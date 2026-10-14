use super::super::{facts::*, AcquisitionReadBudget, CaptureError};
use super::{attempt_prefix, dispatch_prefix, take_fact, terminal_operation};
use crate::{Receipt, StoreError, StoreResult};
use census_domain::model::*;
use fjall::{Keyspace, Readable, Snapshot};
use std::collections::BTreeMap;

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
    if reservation.version != protocol {
        return Err(CaptureError::CaptureProtocolUnsupported.into());
    }
    if &reservation.attempt != id {
        return Err(CaptureError::CorruptClosure.into());
    }
    let dispatches = dispatches(rows, id, protocol, budget)?;
    let prepared: Option<Prepared> =
        take_fact(rows, &format!("{prefix}/prepared"), protocol, budget)?;
    let terminal: Option<ImmutableAttemptResult> =
        take_fact(rows, &format!("{prefix}/terminal"), protocol, budget)?;
    if prepared
        .as_ref()
        .is_some_and(|prepared| prepared.version != protocol)
        || terminal
            .as_ref()
            .is_some_and(|terminal| terminal.protocol() != protocol)
    {
        return Err(CaptureError::CaptureProtocolUnsupported.into());
    }
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
fn dispatches(
    rows: &mut BTreeMap<String, Vec<u8>>,
    id: &AcquisitionAttemptId,
    protocol: CaptureProtocolVersion,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<Vec<(Armed, Option<Observed>)>> {
    let mut dispatches: Vec<(Armed, Option<Observed>)> = Vec::new();
    budget.reserve_recovery_owned(5 * std::mem::size_of::<(Armed, Option<Observed>)>() + 640)?;
    dispatches
        .try_reserve_exact(5)
        .map_err(|_| CaptureError::ReadBudgetExceeded {
            resource: "allocation",
        })?;
    for hop in 0..=4 {
        let copied = match protocol {
            CaptureProtocolVersion::V1 => id.clone(),
            CaptureProtocolVersion::V2 => {
                id.try_clone()
                    .map_err(|_| CaptureError::ReadBudgetExceeded {
                        resource: "allocation",
                    })?
            }
        };
        let dispatch =
            AcquisitionDispatchId::new(copied, HopOrdinal::new(hop).map_err(CaptureError::Domain)?);
        let prefix = dispatch_prefix(&dispatch);
        let Some(armed): Option<Armed> =
            take_fact(rows, &format!("{prefix}/armed"), protocol, budget)?
        else {
            break;
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
        validate_prior(dispatches.last(), &armed)?;
        dispatches.push((armed, response));
    }
    Ok(dispatches)
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

pub(super) fn receipt(
    snapshot: &Snapshot,
    receipts: &Keyspace,
    facts: &AttemptFacts,
    budget: &mut AcquisitionReadBudget,
    protocol: CaptureProtocolVersion,
) -> StoreResult<ReceiptWitness> {
    let operation = terminal_operation(&facts.reservation.attempt, protocol);
    budget.receipt_probe()?;
    let length = snapshot
        .size_of(receipts, operation.as_bytes())
        .map_err(|source| StoreError::Read { source })?;
    if let Some(length) = length {
        let length = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
        if length > MAX_CAPTURE_RECORD_BYTES {
            return Err(CaptureError::RecordTooLarge.into());
        }
        budget.encoded(
            operation
                .len()
                .checked_add(length)
                .ok_or(StoreError::CounterOverflow)?,
        )?;
        budget.present_receipt()?;
    }
    let raw = snapshot
        .get(receipts, operation.as_bytes())
        .map_err(|source| StoreError::Read { source })?;
    match (&facts.terminal, raw) {
        (None, None) => Ok(ReceiptWitness::Absent),
        (Some(terminal), Some(raw)) => decode_receipt_witness(&operation, terminal, &raw, budget),
        _ => Err(CaptureError::CorruptClosure.into()),
    }
}

fn decode_receipt_witness(
    operation: &str,
    terminal: &ImmutableAttemptResult,
    raw: &[u8],
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<ReceiptWitness> {
    if raw.len() > MAX_CAPTURE_RECORD_BYTES {
        return Err(CaptureError::RecordTooLarge.into());
    }
    let receipt: Receipt = {
        crate::acquisition::codec::reserve_decode(raw, budget)?;
        serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
            key: operation.to_string(),
            source,
        })?
    };
    if receipt.operation != operation
        || receipt.digest != digest(terminal)?.as_str()
        || receipt.appended != 0
        || receipt.credited
    {
        return Err(CaptureError::CorruptClosure.into());
    }
    let mut owned = Vec::new();
    budget.reserve_recovery_owned(raw.len())?;
    owned
        .try_reserve_exact(raw.len())
        .map_err(|_| CaptureError::ReadBudgetExceeded {
            resource: "receipt allocation",
        })?;
    owned.extend_from_slice(raw);
    Ok(ReceiptWitness::Present {
        receipt,
        raw: owned,
    })
}
