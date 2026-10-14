use super::super::{facts::*, AcquisitionReadBudget, CaptureError};
use super::{attempt_prefix, dispatch_prefix, take, terminal_operation};
use crate::{Receipt, StoreError, StoreResult};
use census_domain::model::*;
use fjall::{Keyspace, Readable, Snapshot};
use std::collections::BTreeMap;

pub(super) fn attempt(
    rows: &mut BTreeMap<String, Vec<u8>>,
    id: &AcquisitionAttemptId,
) -> StoreResult<Option<AttemptFacts>> {
    let prefix = attempt_prefix(id);
    let Some(reservation): Option<Reservation> = take(rows, &format!("{prefix}/reserved"))? else {
        return Ok(None);
    };
    if &reservation.attempt != id {
        return Err(CaptureError::CorruptClosure.into());
    }
    let mut dispatches: Vec<(Armed, Option<Observed>)> = Vec::new();
    dispatches
        .try_reserve_exact(5)
        .map_err(|_| CaptureError::ReadBudgetExceeded {
            resource: "allocation",
        })?;
    for hop in 0..=4 {
        let dispatch = AcquisitionDispatchId::new(
            id.clone(),
            HopOrdinal::new(hop).map_err(CaptureError::Domain)?,
        );
        let prefix = dispatch_prefix(&dispatch);
        let Some(armed): Option<Armed> = take(rows, &format!("{prefix}/armed"))? else {
            break;
        };
        if armed.dispatch != dispatch {
            return Err(CaptureError::CorruptClosure.into());
        }
        let response: Option<Observed> = take(rows, &format!("{prefix}/response"))?;
        if response
            .as_ref()
            .is_some_and(|r| r.dispatch != dispatch || !r.response.is_for_dispatch(&armed.request))
        {
            return Err(CaptureError::CorruptClosure.into());
        }
        if let Some((_, prior)) = dispatches.last() {
            let prior: &Option<Observed> = prior;
            if !prior
                .as_ref()
                .is_some_and(|r| r.response.redirects_to(&armed.request))
            {
                return Err(CaptureError::CorruptClosure.into());
            }
        }
        if dispatches
            .last()
            .is_some_and(|(prior, _)| prior.request.conditional() != armed.request.conditional())
        {
            return Err(CaptureError::CorruptClosure.into());
        }
        dispatches.push((armed, response));
    }
    let prepared = take(rows, &format!("{prefix}/prepared"))?;
    let terminal = take(rows, &format!("{prefix}/terminal"))?;
    let facts = AttemptFacts {
        reservation,
        dispatches,
        prepared,
        terminal,
    };
    validate(&facts)?;
    Ok(Some(facts))
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
        AcquisitionTerminalV1::ResponseRetained { prepared_digest, disposition } => facts.prepared.as_ref().is_some_and(|p| p.disposition == *disposition) && facts.prepared.as_ref().map(digest).transpose()?.as_ref() == Some(prepared_digest),
        AcquisitionTerminalV1::ValidatedNotModified { dispatch, original } => facts.prepared.is_none() && facts.dispatches.last().is_some_and(|(armed, response)| &armed.dispatch == dispatch && response.as_ref().is_some_and(|r| r.response.status() == 304) && matches!(armed.request.conditional(), ConditionalRequestV1::Pinned { original: pinned, .. } if pinned == original)),
        AcquisitionTerminalV1::NotDispatched { failure } => facts.dispatches.is_empty() && facts.prepared.is_none() && !matches!(failure, AcquisitionFailureV1::ResponseCheckpointRefused { .. }),
        AcquisitionTerminalV1::FailedUncaptured { checkpoint, failure } => !facts.dispatches.is_empty() && facts.prepared.is_none() && checkpoint == &facts.checkpoint() && ((matches!(checkpoint, AcquisitionCheckpointV1::ResponseObserved { .. }) && !matches!(failure, AcquisitionFailureV1::ResponseCheckpointRefused { .. })) || (matches!(checkpoint, AcquisitionCheckpointV1::Armed { .. }) && matches!(failure, AcquisitionFailureV1::ResponseCheckpointRefused { response_url, .. } if facts.dispatches.last().is_some_and(|(armed, _)| response_url.is_for_dispatch(&armed.request))))),
        AcquisitionTerminalV1::OutcomeUnknown { checkpoint, .. } => !facts.dispatches.is_empty() && facts.prepared.is_none() && checkpoint == &facts.checkpoint(),
    };
    if !valid {
        return Err(CaptureError::CorruptClosure.into());
    }
    Ok(())
}

pub(super) fn work(facts: &WorkFacts) -> StoreResult<()> {
    let work_id = facts.work.id().map_err(CaptureError::Domain)?;
    for (index, attempt) in facts.attempts.iter().enumerate() {
        if index.checked_add(1).is_some_and(|position| position < facts.attempts.len()) && !attempt.terminal.as_ref().is_some_and(|t| matches!(t.terminal(), AcquisitionTerminalV1::OutcomeUnknown { .. } | AcquisitionTerminalV1::ResponseRetained { disposition: CaptureDispositionV1::RetryableResponse, .. }) || matches!(t.terminal(), AcquisitionTerminalV1::FailedUncaptured { failure, .. } if failure.allows_continuation())) { return Err(CaptureError::CorruptClosure.into()); }
        if attempt
            .dispatches
            .first()
            .is_some_and(|(a, _)| a.request.target_url() != facts.work.request().original_url())
        {
            return Err(CaptureError::CorruptClosure.into());
        }
        if let Some(terminal) = &attempt.terminal {
            if matches!(
                terminal.terminal(),
                AcquisitionTerminalV1::ResponseRetained {
                    disposition: CaptureDispositionV1::ReusableResponse
                        | CaptureDispositionV1::TerminalResponse,
                    ..
                } | AcquisitionTerminalV1::ValidatedNotModified { .. }
            ) && facts.result.as_ref().map(|r| &r.occurrence) != Some(&terminal.occurrence()?)
            {
                return Err(CaptureError::CorruptClosure.into());
            }
        }
    }
    if let Some(result) = &facts.result {
        if result.occurrence.attempt().store_identity() != facts.work.slot().store_identity() {
            return Err(CaptureError::CaptureCapabilityForeignStore.into());
        }
        if result.occurrence.attempt().work_id() == &work_id {
            if !facts.attempts.iter().any(|a| {
                a.terminal.as_ref().is_some_and(|t| {
                    !matches!(
                        t.terminal(),
                        AcquisitionTerminalV1::ResponseRetained {
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

pub(super) fn receipt(
    snapshot: &Snapshot,
    receipts: &Keyspace,
    facts: &AttemptFacts,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<()> {
    let operation = terminal_operation(&facts.reservation.attempt);
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
    }
    let raw = snapshot
        .get(receipts, operation.as_bytes())
        .map_err(|source| StoreError::Read { source })?;
    match (&facts.terminal, raw) {
        (None, None) => Ok(()),
        (Some(terminal), Some(raw)) => {
            if raw.len() > MAX_CAPTURE_RECORD_BYTES {
                return Err(CaptureError::RecordTooLarge.into());
            }
            let receipt: Receipt =
                serde_json::from_slice(&raw).map_err(|source| StoreError::Decode {
                    key: operation.clone(),
                    source,
                })?;
            if receipt.operation != operation
                || receipt.digest != digest(terminal)?.as_str()
                || receipt.appended != 0
                || receipt.credited
            {
                return Err(CaptureError::CorruptClosure.into());
            }
            Ok(())
        }
        _ => Err(CaptureError::CorruptClosure.into()),
    }
}
