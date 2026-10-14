use super::{read_work, VerifiedRetainedOccurrence};
use crate::acquisition::{facts::*, AcquisitionReadBudget, CaptureError};
use crate::StoreResult;
use census_domain::model::*;
use fjall::{Keyspace, Snapshot};
use std::path::Path;

pub(crate) fn verify_reference(
    root: &Path,
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    reference: &AcquisitionOccurrenceRefV1,
    budget: &mut AcquisitionReadBudget,
    protocol: CaptureProtocolVersion,
) -> StoreResult<VerifiedRetainedOccurrence> {
    let facts = read_work(
        snapshot,
        journal,
        receipts,
        reference.attempt().work_id(),
        budget,
        protocol,
    )?;
    verify_from_facts(root, snapshot, journal, receipts, &facts, reference, budget)
}
pub(in crate::acquisition) fn verify_result(
    root: &Path,
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    facts: &WorkFacts,
    reference: &AcquisitionOccurrenceRefV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<VerifiedRetainedOccurrence> {
    if reference.attempt().work_id() == &facts.work.id().map_err(CaptureError::Domain)? {
        verify_from_facts(root, snapshot, journal, receipts, facts, reference, budget)
    } else {
        let original = read_work(
            snapshot,
            journal,
            receipts,
            reference.attempt().work_id(),
            budget,
            facts.protocol,
        )?;
        if original.work.request() != facts.work.request()
            || reference.attempt().store_identity() != facts.work.slot().store_identity()
        {
            return Err(CaptureError::CacheOccurrenceMismatch.into());
        }
        reusable_body_reference(root, &original, reference, budget)
    }
}

fn verify_from_facts(
    root: &Path,
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    facts: &WorkFacts,
    reference: &AcquisitionOccurrenceRefV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<VerifiedRetainedOccurrence> {
    if facts.work.slot().store_identity() != reference.attempt().store_identity() {
        return Err(CaptureError::CaptureCapabilityForeignStore.into());
    }
    let attempt = facts
        .attempts
        .iter()
        .find(|a| &a.reservation.attempt == reference.attempt())
        .ok_or(CaptureError::CacheOccurrenceMismatch)?;
    let terminal = attempt
        .terminal
        .as_ref()
        .ok_or(CaptureError::RecoveryRequired)?;
    if terminal.occurrence()? != *reference {
        return Err(CaptureError::CacheOccurrenceMismatch.into());
    }
    match terminal.terminal() {
        AcquisitionTerminal::ResponseRetained { .. } => {
            retained_body(root, facts, attempt, reference, budget)
        }
        AcquisitionTerminal::ValidatedNotModified { original, .. } => {
            if original.attempt().work_id() == reference.attempt().work_id() {
                return Err(CaptureError::CorruptClosure.into());
            }
            let original_facts = read_work(
                snapshot,
                journal,
                receipts,
                original.attempt().work_id(),
                budget,
                facts.protocol,
            )?;
            if original_facts.work.request() != facts.work.request()
                || original.attempt().store_identity() != facts.work.slot().store_identity()
            {
                return Err(CaptureError::CacheOccurrenceMismatch.into());
            }
            let mut verified = reusable_body_reference(root, &original_facts, original, budget)?;
            verified.reference = reference.clone();
            verified.revalidation = Some(last_response(attempt)?.clone());
            Ok(verified)
        }
        _ => Err(CaptureError::CacheOccurrenceMismatch.into()),
    }
}
fn retained_body(
    root: &Path,
    facts: &WorkFacts,
    attempt: &AttemptFacts,
    reference: &AcquisitionOccurrenceRefV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<VerifiedRetainedOccurrence> {
    let prepared = attempt
        .prepared
        .as_ref()
        .ok_or(CaptureError::CorruptClosure)?;
    let response = last_response(attempt)?;
    let body_path =
        crate::acquisition::backing::verify(root, &facts.work, attempt, &prepared.capture, budget)?;
    Ok(VerifiedRetainedOccurrence {
        reference: reference.clone(),
        body_reference: reference.clone(),
        prepared: prepared.capture.clone(),
        response: response.clone(),
        body_path,
        revalidation: None,
    })
}
pub(in crate::acquisition) fn last_response(
    facts: &AttemptFacts,
) -> StoreResult<&ResponseObservationV1> {
    facts
        .dispatches
        .last()
        .and_then(|(_, response)| response.as_ref())
        .map(|response| &response.response)
        .ok_or_else(|| CaptureError::CorruptClosure.into())
}

pub(in crate::acquisition) fn reusable_body_reference(
    root: &Path,
    facts: &WorkFacts,
    reference: &AcquisitionOccurrenceRefV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<VerifiedRetainedOccurrence> {
    if facts.work.slot().store_identity() != reference.attempt().store_identity() {
        return Err(CaptureError::CaptureCapabilityForeignStore.into());
    }
    let attempt = facts
        .attempts
        .iter()
        .find(|a| &a.reservation.attempt == reference.attempt())
        .ok_or(CaptureError::CacheOccurrenceMismatch)?;
    let terminal = attempt
        .terminal
        .as_ref()
        .ok_or(CaptureError::RecoveryRequired)?;
    if !matches!(
        terminal.terminal(),
        AcquisitionTerminal::ResponseRetained {
            disposition: CaptureDispositionV1::ReusableResponse,
            ..
        }
    ) || terminal.occurrence()? != *reference
    {
        return Err(CaptureError::CacheOccurrenceMismatch.into());
    }
    retained_body(root, facts, attempt, reference, budget)
}
