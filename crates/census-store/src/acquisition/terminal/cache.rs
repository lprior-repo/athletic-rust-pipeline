use crate::acquisition::{
    codec, facts::*, read, AcquisitionReadBudget, CaptureError, OwnedAcquisitionWork,
    VerifiedRetainedOccurrence,
};
use crate::{Store, StoreResult};
use census_domain::model::*;

impl Store {
    pub fn reuse_acquisition_cache(
        &self,
        work: OwnedAcquisitionWork,
        original: &AcquisitionOccurrenceRefV1,
        budget: &mut AcquisitionReadBudget,
    ) -> StoreResult<VerifiedRetainedOccurrence> {
        budget.begin_operation();
        let _writes = self.lock_appends();
        work.verify(self)?;
        let snapshot = self.db.snapshot();
        let facts = read::read_work(
            &snapshot,
            &self.journal,
            &self.receipts,
            &work.id,
            budget,
            work.owner.protocol(),
        )?;
        if facts.work != work.work || facts.work.cache_intent() != CaptureCacheIntent::ReuseEligible
        {
            return Err(CaptureError::CacheOccurrenceMismatch.into());
        }
        if let Some(result) = &facts.result {
            if &result.occurrence != original {
                return Err(CaptureError::AttemptTerminalConflict.into());
            }
            let verified = read::verify_result(
                self.root(),
                &snapshot,
                &self.journal,
                &self.receipts,
                &facts,
                original,
                budget,
            )?;
            codec::repeat(self)?;
            return Ok(verified);
        }
        if facts
            .attempts
            .iter()
            .any(|a| !a.dispatches.is_empty() || a.terminal.is_some())
            || original.attempt().work_id() == &work.id
        {
            return Err(CaptureError::TransitionRefused.into());
        }
        let verified = reusable_original(self, &snapshot, &facts, original, budget)?;
        commit_cache_reuse(self, &work.id, original, facts.protocol)?;
        Ok(verified)
    }
}

fn commit_cache_reuse(
    store: &Store,
    work_id: &AcquisitionWorkId,
    original: &AcquisitionOccurrenceRefV1,
    protocol: CaptureProtocolVersion,
) -> StoreResult<()> {
    let mut batch = store.db.batch();
    codec::stage(
        store,
        &mut batch,
        &format!("work/{}/result", work_id),
        &WorkResult {
            version: protocol,
            occurrence: original.clone(),
        },
        protocol,
    )?;
    codec::commit(batch)
}

fn reusable_original(
    store: &Store,
    snapshot: &fjall::Snapshot,
    facts: &WorkFacts,
    original: &AcquisitionOccurrenceRefV1,
    budget: &mut AcquisitionReadBudget,
) -> StoreResult<VerifiedRetainedOccurrence> {
    let original_facts = read::read_work(
        snapshot,
        &store.journal,
        &store.receipts,
        original.attempt().work_id(),
        budget,
        facts.protocol,
    )?;
    if facts.work.request() != original_facts.work.request()
        || original.attempt().store_identity() != facts.work.slot().store_identity()
    {
        return Err(CaptureError::CacheOccurrenceMismatch.into());
    }
    read::reusable_body_reference(store.root(), &original_facts, original, budget)
}
