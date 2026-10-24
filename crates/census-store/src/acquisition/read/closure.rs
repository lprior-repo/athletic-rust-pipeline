use super::super::{facts::*, AcquisitionReadBudget, CaptureError};
use super::{attempt_prefix, dispatch_prefix, take_fact};
use crate::{StoreError, StoreResult};
use census_domain::model::*;
use std::collections::BTreeMap;

mod receipt;
pub(super) use receipt::receipt;

pub(super) fn attempt(
    rows: &mut BTreeMap<String, Vec<u8>>,
    id: &AcquisitionAttemptId,
    protocol: CaptureProtocolVersion,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Option<AttemptFacts>> {
    let prefix = attempt_prefix(id);
    let Some(reservation): Option<Reservation> =
        take_fact(rows, &format!("{prefix}/reserved"), protocol, budget)?
    else {
        return Ok(None);
    };
    validate_reservation_protocol(&reservation, protocol)?;
    if &reservation.attempt != id {
        return Err(CaptureError::CorruptClosure.into());
    }
    let dispatches = dispatches(rows, id, protocol, budget)?;
    let prepared: Option<Prepared> =
        take_fact(rows, &format!("{prefix}/prepared"), protocol, budget)?;
    let terminal: Option<ImmutableAttemptResult> =
        take_fact(rows, &format!("{prefix}/terminal"), protocol, budget)?;
    validate_terminal_protocols(&prepared, &terminal, protocol)?;
    let facts = AttemptFacts {
        reservation,
        dispatches,
        prepared,
        terminal,
        receipt: ReceiptWitness::Absent,
    };
    validate(&facts)?;
    Ok(Some(facts))
}
fn validate_reservation_protocol(
    reservation: &Reservation,
    protocol: CaptureProtocolVersion,
) -> StoreResult<()> {
    if reservation.version != protocol {
        return Err(CaptureError::CaptureProtocolUnsupported.into());
    }
    Ok(())
}
fn validate_terminal_protocols(
    prepared: &Option<Prepared>,
    terminal: &Option<ImmutableAttemptResult>,
    protocol: CaptureProtocolVersion,
) -> StoreResult<()> {
    if prepared.as_ref().is_some_and(|p| p.version != protocol)
        || terminal.as_ref().is_some_and(|t| t.protocol() != protocol)
    {
        return Err(CaptureError::CaptureProtocolUnsupported.into());
    }
    Ok(())
}
const DISPATCH_CLOSURE_HINT_BYTES: usize = 5 * std::mem::size_of::<(Armed, Option<Observed>)>();

fn dispatches(
    rows: &mut BTreeMap<String, Vec<u8>>,
    id: &AcquisitionAttemptId,
    protocol: CaptureProtocolVersion,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Vec<(Armed, Option<Observed>)>> {
    let mut dispatches: Vec<(Armed, Option<Observed>)> = Vec::new();
    let bytes = std::mem::size_of::<[(Armed, Option<Observed>); 5]>()
        .checked_add(640)
        .ok_or(StoreError::CounterOverflow)?;
    budget.reserve_recovery_owned(bytes)?;
    dispatches.try_reserve_exact(5).map_err(|source| {
        CaptureError::allocation_reservation_failed(
            "dispatch closure buffer",
            Some(DISPATCH_CLOSURE_HINT_BYTES),
            source,
        )
    })?;
    for hop in 0..=4 {
        match load_one_dispatch(rows, id, hop, protocol, budget)? {
            Some((armed, response)) => {
                validate_prior(dispatches.last(), &armed)?;
                dispatches.push((armed, response));
            }
            None => break,
        }
    }
    Ok(dispatches)
}

fn load_one_dispatch(
    rows: &mut BTreeMap<String, Vec<u8>>,
    id: &AcquisitionAttemptId,
    hop: u8,
    protocol: CaptureProtocolVersion,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Option<(Armed, Option<Observed>)>> {
    let copied = match protocol {
        CaptureProtocolVersion::V1 => id.clone(),
        CaptureProtocolVersion::V2 => id.try_clone().map_err(|source| {
            CaptureError::allocation_reservation_failed("dispatch identity clone", None, source)
        })?,
    };
    let dispatch =
        AcquisitionDispatchId::new(copied, HopOrdinal::new(hop).map_err(CaptureError::Domain)?);
    let prefix = dispatch_prefix(&dispatch);
    let Some(armed): Option<Armed> = take_fact(rows, &format!("{prefix}/armed"), protocol, budget)?
    else {
        return Ok(None);
    };
    if armed.version != protocol {
        return Err(CaptureError::CaptureProtocolUnsupported.into());
    }
    if armed.dispatch != dispatch {
        return Err(CaptureError::CorruptClosure.into());
    }
    let response: Option<Observed> =
        take_fact(rows, &format!("{prefix}/response"), protocol, budget)?;
    if response
        .as_ref()
        .is_some_and(|response| response.version != protocol)
    {
        return Err(CaptureError::CaptureProtocolUnsupported.into());
    }
    if response
        .as_ref()
        .is_some_and(|r| r.dispatch != dispatch || !r.response.is_for_dispatch(&armed.request))
    {
        return Err(CaptureError::CorruptClosure.into());
    }
    Ok(Some((armed, response)))
}
fn validate_prior(
    prior_pair: Option<&(Armed, Option<Observed>)>,
    armed: &Armed,
) -> StoreResult<()> {
    if let Some((_, prior)) = prior_pair {
        let prior: &Option<Observed> = prior;
        if !prior
            .as_ref()
            .is_some_and(|r| r.response.redirects_to(&armed.request))
        {
            return Err(CaptureError::CorruptClosure.into());
        }
    }
    if prior_pair
        .is_some_and(|(prior, _)| prior.request.conditional() != armed.request.conditional())
    {
        return Err(CaptureError::CorruptClosure.into());
    }
    Ok(())
}
fn validate(facts: &AttemptFacts) -> StoreResult<()> {
    if let Some(prepared) = &facts.prepared {
        let (last, Some(response)) = facts
            .dispatches
            .last()
            .ok_or(CaptureError::CorruptClosure)?
        else {
            return Err(CaptureError::CorruptClosure.into());
        };
        if CaptureDispositionV1::for_http_status(response.response.status())
            .map_err(CaptureError::Domain)?
            != prepared.disposition
        {
            return Err(CaptureError::CorruptClosure.into());
        }
        if prepared.attempt != facts.reservation.attempt
            || prepared.dispatch != last.dispatch
            || response.response.is_redirect()
            || response.response.status() == 304
        {
            return Err(CaptureError::CorruptClosure.into());
        }
    }
    let Some(terminal) = &facts.terminal else {
        return Ok(());
    };
    if terminal.attempt() != &facts.reservation.attempt {
        return Err(CaptureError::CorruptClosure.into());
    }
    let valid = match terminal.terminal() {
        AcquisitionTerminal::PreparedRetentionFailed { prepared_digest, decision } => {
            terminal.protocol() == CaptureProtocolVersion::V2
                && facts.prepared.as_ref().is_some_and(|prepared| prepared.capture.content_bytes() != 0)
                && facts.prepared.as_ref().map(digest).transpose()?.as_ref() == Some(prepared_digest)
                && decision.attempt() == terminal.attempt()
                && decision.store_identity() == terminal.attempt().store_identity()
                && decision.prepared_digest() == prepared_digest
        }
        AcquisitionTerminal::ResponseRetained { prepared_digest, disposition } => facts.prepared.as_ref().is_some_and(|p| p.disposition == *disposition) && facts.prepared.as_ref().map(digest).transpose()?.as_ref() == Some(prepared_digest),
        AcquisitionTerminal::ValidatedNotModified { dispatch, original } => facts.prepared.is_none() && facts.dispatches.last().is_some_and(|(armed, response)| &armed.dispatch == dispatch && response.as_ref().is_some_and(|r| r.response.status() == 304) && matches!(armed.request.conditional(), ConditionalRequestV1::Pinned { original: pinned, .. } if pinned == original)),
        AcquisitionTerminal::NotDispatched { failure } => facts.dispatches.is_empty() && facts.prepared.is_none() && !matches!(failure, AcquisitionFailureV1::ResponseCheckpointRefused { .. }),
        AcquisitionTerminal::FailedUncaptured { checkpoint, failure } => !facts.dispatches.is_empty() && facts.prepared.is_none() && checkpoint == &facts.checkpoint() && ((matches!(checkpoint, AcquisitionCheckpointV1::ResponseObserved { .. }) && !matches!(failure, AcquisitionFailureV1::ResponseCheckpointRefused { .. })) || (matches!(checkpoint, AcquisitionCheckpointV1::Armed { .. }) && matches!(failure, AcquisitionFailureV1::ResponseCheckpointRefused { response_url, .. } if facts.dispatches.last().is_some_and(|(armed, _)| response_url.is_for_dispatch(&armed.request))))),
        AcquisitionTerminal::OutcomeUnknown { checkpoint, .. } => !facts.dispatches.is_empty() && facts.prepared.is_none() && checkpoint == &facts.checkpoint(),
    };
    if !valid {
        return Err(CaptureError::CorruptClosure.into());
    }
    Ok(())
}

pub(super) fn work(facts: &WorkFacts) -> StoreResult<()> {
    let work_id = facts.work.id().map_err(CaptureError::Domain)?;
    if facts
        .result
        .as_ref()
        .is_some_and(|result| result.version != facts.protocol)
    {
        return Err(CaptureError::CaptureProtocolUnsupported.into());
    }
    for (index, attempt) in facts.attempts.iter().enumerate() {
        crate::acquisition::observed::prefix(&facts.work, attempt, None, None)?;
        for pair in attempt.dispatches.windows(2) {
            let [(_, Some(prior)), (next, _)] = pair else {
                return Err(CaptureError::CorruptClosure.into());
            };
            crate::acquisition::observed::redirect(&facts.work, &prior.response, &next.request)?;
        }
        for (armed, observed) in &attempt.dispatches {
            crate::acquisition::observed::request(&facts.work, &armed.request)?;
            if let Some(observed) = observed {
                crate::acquisition::observed::response(
                    &facts.work,
                    &armed.dispatch,
                    &armed.request,
                    &observed.response,
                )?;
            }
        }
        if index.checked_add(1).is_some_and(|position| position < facts.attempts.len()) && !attempt.terminal.as_ref().is_some_and(|t| matches!(t.terminal(), AcquisitionTerminal::PreparedRetentionFailed { .. } | AcquisitionTerminal::OutcomeUnknown { .. } | AcquisitionTerminal::ResponseRetained { disposition: CaptureDispositionV1::RetryableResponse, .. }) || matches!(t.terminal(), AcquisitionTerminal::FailedUncaptured { failure, .. } if failure.allows_continuation())) { return Err(CaptureError::CorruptClosure.into()); }
        if attempt
            .dispatches
            .first()
            .is_some_and(|(a, _)| a.request.target_url() != facts.work.request().original_url())
        {
            return Err(CaptureError::CorruptClosure.into());
        }
        validate_terminal_result(facts, attempt)?;
    }
    validate_work_result(facts, &work_id)
}

fn validate_work_result(facts: &WorkFacts, work_id: &AcquisitionWorkId) -> StoreResult<()> {
    if let Some(result) = &facts.result {
        if result.occurrence.attempt().store_identity() != facts.work.slot().store_identity() {
            return Err(CaptureError::CaptureCapabilityForeignStore.into());
        }
        if result.occurrence.attempt().work_id() == work_id {
            if !facts.attempts.iter().any(|a| {
                a.terminal.as_ref().is_some_and(|t| {
                    !matches!(
                        t.terminal(),
                        AcquisitionTerminal::ResponseRetained {
                            disposition: CaptureDispositionV1::RetryableResponse,
                            ..
                        }
                    ) && t.occurrence().is_ok_and(|r| r == result.occurrence)
                })
            }) {
                return Err(CaptureError::CorruptClosure.into());
            }
        } else if facts.work.cache_intent() != CaptureCacheIntent::ReuseEligible
            || facts
                .attempts
                .iter()
                .any(|a| !a.dispatches.is_empty() || a.terminal.is_some())
        {
            return Err(CaptureError::CorruptClosure.into());
        }
    }
    Ok(())
}

fn validate_terminal_result(facts: &WorkFacts, attempt: &AttemptFacts) -> StoreResult<()> {
    if let Some(terminal) = &attempt.terminal {
        if let AcquisitionTerminal::PreparedRetentionFailed { decision, .. } = terminal.terminal() {
            if decision.slot_id() != &facts.work.slot().id().map_err(CaptureError::Domain)? {
                return Err(CaptureError::CorruptClosure.into());
            }
            let prepared = attempt
                .prepared
                .as_ref()
                .ok_or(CaptureError::CorruptClosure)?;
            crate::acquisition::backing::validate_metadata(
                &facts.work,
                attempt,
                &prepared.capture,
            )?;
        }
        if matches!(
            terminal.terminal(),
            AcquisitionTerminal::ResponseRetained {
                disposition: CaptureDispositionV1::ReusableResponse
                    | CaptureDispositionV1::TerminalResponse,
                ..
            } | AcquisitionTerminal::ValidatedNotModified { .. }
        ) && facts.result.as_ref().map(|r| &r.occurrence) != Some(&terminal.occurrence()?)
        {
            return Err(CaptureError::CorruptClosure.into());
        }
    }
    Ok(())
}
