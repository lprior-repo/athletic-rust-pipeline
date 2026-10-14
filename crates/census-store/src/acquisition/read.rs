use super::{codec, facts::*, AcquisitionReadBudget, CaptureError, OwnedAcquisitionWork};
use crate::{Store, StoreError, StoreResult};
use census_domain::model::*;
use fjall::{Keyspace, Readable, Snapshot};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

mod closure;
mod retained;
use retained::verify_reference;
pub(super) use retained::{last_response, reusable_body_reference, verify_result};

pub struct VerifiedRetainedOccurrence {
    pub(super) reference: AcquisitionOccurrenceRefV1,
    pub(super) body_reference: AcquisitionOccurrenceRefV1,
    pub(super) prepared: PreparedCaptureInputV1,
    pub(super) response: ResponseObservationV1,
    pub(super) body_path: PathBuf,
    pub(super) revalidation: Option<ResponseObservationV1>,
}
impl VerifiedRetainedOccurrence {
    pub fn reference(&self) -> &AcquisitionOccurrenceRefV1 {
        &self.reference
    }
    pub fn body_reference(&self) -> &AcquisitionOccurrenceRefV1 {
        &self.body_reference
    }
    pub fn prepared(&self) -> &PreparedCaptureInputV1 {
        &self.prepared
    }
    pub fn response(&self) -> &ResponseObservationV1 {
        &self.response
    }
    pub fn body_path(&self) -> &Path {
        &self.body_path
    }
    pub fn revalidation(&self) -> Option<&ResponseObservationV1> {
        self.revalidation.as_ref()
    }
}

impl Store {
    pub fn inspect_acquisition_work(
        &self,
        work: &OwnedAcquisitionWork,
        budget: &mut AcquisitionReadBudget,
    ) -> StoreResult<AcquireDecision> {
        budget.begin_operation();
        let _writes = self.lock_appends();
        work.verify(self)?;
        let snapshot = self.db.snapshot();
        let facts = read_work(&snapshot, &self.journal, &self.receipts, &work.id, budget)?;
        if facts.work != work.work {
            return Err(CaptureError::ObligationConflict.into());
        }
        if let Some(result) = &facts.result {
            verify_result(
                self.root(),
                &snapshot,
                &self.journal,
                &self.receipts,
                &facts,
                &result.occurrence,
                budget,
            )?;
        }
        Ok(facts.decision())
    }
    pub fn read_retained_acquisition_v1(
        &self,
        reference: &AcquisitionOccurrenceRefV1,
        budget: &mut AcquisitionReadBudget,
    ) -> StoreResult<VerifiedRetainedOccurrence> {
        budget.begin_operation();
        let _writes = self.lock_appends();
        let snapshot = self.db.snapshot();
        let declaration = super::qualification::declaration(
            &snapshot,
            &self.meta,
            &self.journal,
            &self.store_format()?,
        )?
        .ok_or(CaptureError::CaptureCapabilityMissing)?;
        if declaration.store_identity() != reference.attempt().store_identity() {
            return Err(CaptureError::CaptureCapabilityForeignStore.into());
        }
        verify_reference(
            self.root(),
            &snapshot,
            &self.journal,
            &self.receipts,
            reference,
            budget,
        )
    }
}

pub(super) fn read_work(
    snapshot: &Snapshot,
    journal: &Keyspace,
    receipts: &Keyspace,
    id: &AcquisitionWorkId,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<WorkFacts> {
    let prefix = format!("work/{id}/");
    let physical_prefix = Store::journal_key(SOURCE_ACQUISITION_PHASE, &prefix);
    let mut rows = BTreeMap::new();
    for (position, guard) in snapshot
        .prefix(journal, &physical_prefix)
        .take(42)
        .enumerate()
    {
        if position == 41 {
            return Err(CaptureError::UnsupportedRoot.into());
        }
        let physical = guard.key().map_err(|source| StoreError::Read { source })?;
        if physical.len() > crate::MAX_JOURNAL_KEY_BYTES {
            return Err(CaptureError::UnsupportedRoot.into());
        }
        let logical = physical
            .strip_prefix(Store::journal_key(SOURCE_ACQUISITION_PHASE, "").as_slice())
            .ok_or(CaptureError::UnsupportedRoot)?;
        let key = std::str::from_utf8(logical)
            .map_err(|_| CaptureError::UnsupportedRoot)?
            .to_string();
        if key.as_bytes().contains(&0) || key.as_bytes().contains(&1) {
            return Err(CaptureError::UnsupportedRoot.into());
        }
        let raw =
            codec::raw(snapshot, journal, &key, budget)?.ok_or(CaptureError::CorruptClosure)?;
        rows.insert(key, raw);
    }
    let work: AcquisitionWorkV1 =
        take(&mut rows, &format!("{prefix}identity"))?.ok_or(CaptureError::SlotWorkMismatch)?;
    if work.id().map_err(CaptureError::Domain)? != *id {
        return Err(CaptureError::WorkDigestMismatch.into());
    }
    let slot_key = format!(
        "slot/{}/binding",
        work.slot().id().map_err(CaptureError::Domain)?
    );
    let binding: SlotBinding =
        codec::get(snapshot, journal, &slot_key, budget)?.ok_or(CaptureError::SlotWorkMismatch)?;
    if binding.slot != *work.slot() || binding.work_id != *id {
        return Err(CaptureError::SlotWorkMismatch.into());
    }
    let mut attempts = Vec::new();
    attempts
        .try_reserve_exact(3)
        .map_err(|_| CaptureError::ReadBudgetExceeded {
            resource: "allocation",
        })?;
    for ordinal in 1..=3 {
        let ordinal = AttemptOrdinal::new(ordinal).map_err(CaptureError::Domain)?;
        let attempt =
            AcquisitionAttemptId::new(work.slot().store_identity().clone(), id.clone(), ordinal);
        let Some(facts) = closure::attempt(&mut rows, &attempt)? else {
            break;
        };
        closure::receipt(snapshot, receipts, &facts, budget)?;
        attempts.push(facts);
    }
    let result = take(&mut rows, &format!("{prefix}result"))?;
    if !rows.is_empty() {
        return Err(CaptureError::UnsupportedRoot.into());
    }
    let facts = WorkFacts {
        work,
        attempts,
        result,
    };
    closure::work(&facts)?;
    Ok(facts)
}

pub(super) fn take<T: DeserializeOwned>(
    rows: &mut BTreeMap<String, Vec<u8>>,
    key: &str,
) -> StoreResult<Option<T>> {
    rows.remove(key)
        .map(|raw| codec::decode(key, &raw))
        .transpose()
}
pub(super) fn attempt_prefix(id: &AcquisitionAttemptId) -> String {
    format!("work/{}/attempt/{}", id.work_id(), id.ordinal().get())
}
pub(super) fn dispatch_prefix(id: &AcquisitionDispatchId) -> String {
    format!(
        "{}/dispatch/{}",
        attempt_prefix(id.attempt()),
        id.hop().get()
    )
}
pub(super) fn terminal_operation(id: &AcquisitionAttemptId) -> String {
    format!("{SOURCE_ACQUISITION_PHASE}/{}/terminal", attempt_prefix(id))
}
