use super::capability::*;
use super::{codec, facts::*, read, transition, AcquisitionReadBudget, CaptureError};
use crate::{Store, StoreError, StoreResult};
use census_domain::model::*;

mod cache;
mod failure;

impl Store {
    pub fn finish_acquisition_capture(
        &self,
        prepared: PreparedCapture,
    ) -> StoreResult<AcquisitionOccurrenceRefV1> {
        let _writes = self.lock_appends();
        let mut budget = AcquisitionReadBudget::for_work();
        let snapshot = self.db.snapshot();
        let facts = transition::current(self, &prepared.work, &snapshot, &mut budget)?;
        let retained = prepared_attempt(&facts, &prepared.prepared.attempt)?;
        if retained.prepared.as_ref() != Some(&prepared.prepared) {
            return Err(CaptureError::AttemptTerminalConflict.into());
        }
        super::backing::verify(
            self.root(),
            &facts.work,
            retained,
            &prepared.prepared.capture,
            &mut budget,
        )?;
        retained_terminal(self, prepared.prepared)
    }

    pub fn finish_prepared_acquisition(
        &self,
        work: OwnedAcquisitionWork,
        budget: &mut AcquisitionReadBudget,
    ) -> StoreResult<AcquisitionOccurrenceRefV1> {
        if work.protocol() == CaptureProtocolVersion::V2 {
            return Err(CaptureError::PreparedTargetMismatch.into());
        }
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
        if facts.work != work.work {
            return Err(CaptureError::ObligationConflict.into());
        }
        if let Some(result) = &facts.result {
            read::verify_result(
                self.root(),
                &snapshot,
                &self.journal,
                &self.receipts,
                &facts,
                &result.occurrence,
                budget,
            )?;
            self.db
                .persist(fjall::PersistMode::SyncData)
                .map_err(|source| StoreError::Flush { source })?;
            return Ok(result.occurrence.clone());
        }
        let last = facts
            .attempts
            .last()
            .ok_or(CaptureError::PreparedRecoveryRequired)?;
        let prepared = last
            .prepared
            .as_ref()
            .ok_or(CaptureError::PreparedRecoveryRequired)?;
        prepared_attempt(&facts, &prepared.attempt)?;
        super::backing::verify(self.root(), &facts.work, last, &prepared.capture, budget)?;
        retained_terminal(self, prepared.clone())
    }

    pub fn finish_acquisition_not_modified(
        &self,
        response: ObservedResponse,
        original: &AcquisitionOccurrenceRefV1,
        budget: &mut AcquisitionReadBudget,
    ) -> StoreResult<AcquisitionOccurrenceRefV1> {
        budget.begin_operation();
        let _writes = self.lock_appends();
        response.work.verify(self)?;
        let snapshot = self.db.snapshot();
        let facts = read::read_work(
            &snapshot,
            &self.journal,
            &self.receipts,
            &response.work.id,
            budget,
            response.work.owner.protocol(),
        )?;
        let last = transition::observed(&facts, &response.observed)?;
        if response.observed.response.status() != 304
            || original.attempt().work_id() == &response.work.id
        {
            return Err(CaptureError::CacheOccurrenceMismatch.into());
        }
        let armed = last
            .dispatches
            .last()
            .map(|(armed, _)| armed)
            .ok_or(CaptureError::CorruptClosure)?;
        if !matches!(armed.request.conditional(), ConditionalRequestV1::Pinned { original: pinned, .. } if pinned == original)
        {
            return Err(CaptureError::CacheOccurrenceMismatch.into());
        }
        let original_facts = read::read_work(
            &snapshot,
            &self.journal,
            &self.receipts,
            original.attempt().work_id(),
            budget,
            facts.protocol,
        )?;
        if facts.work.request() != original_facts.work.request()
            || facts.work.slot().store_identity() != original.attempt().store_identity()
        {
            return Err(CaptureError::CacheOccurrenceMismatch.into());
        }
        read::reusable_body_reference(self.root(), &original_facts, original, budget)?;
        let terminal = ImmutableAttemptResult::new(
            facts.protocol,
            response.observed.dispatch.attempt().clone(),
            AcquisitionTerminal::ValidatedNotModified {
                dispatch: response.observed.dispatch,
                original: original.clone(),
            },
        )?;
        let occurrence = terminal.occurrence()?;
        settle(self, terminal, Some(&occurrence))?;
        Ok(occurrence)
    }
}

fn prepared_attempt<'a>(
    facts: &'a WorkFacts,
    attempt: &AcquisitionAttemptId,
) -> StoreResult<&'a AttemptFacts> {
    let last = facts
        .attempts
        .last()
        .ok_or(CaptureError::PreparedRecoveryRequired)?;
    if &last.reservation.attempt != attempt
        || last.prepared.is_none()
        || last.terminal.is_some()
        || facts.result.is_some()
    {
        return Err(CaptureError::PreparedRecoveryRequired.into());
    }
    Ok(last)
}
fn retained_terminal(store: &Store, prepared: Prepared) -> StoreResult<AcquisitionOccurrenceRefV1> {
    let terminal = ImmutableAttemptResult::new(
        prepared.version,
        prepared.attempt.clone(),
        AcquisitionTerminal::ResponseRetained {
            prepared_digest: digest(&prepared)?,
            disposition: prepared.disposition,
        },
    )?;
    let occurrence = terminal.occurrence()?;
    let settled = match prepared.disposition {
        CaptureDispositionV1::RetryableResponse => None,
        _ => Some(&occurrence),
    };
    settle(store, terminal, settled)?;
    Ok(occurrence)
}
fn settle(
    store: &Store,
    terminal: ImmutableAttemptResult,
    result: Option<&AcquisitionOccurrenceRefV1>,
) -> StoreResult<ImmutableAttemptResult> {
    let protocol = terminal.protocol();
    let operation = read::terminal_operation(terminal.attempt(), protocol);
    let digest = digest(&terminal)?;
    let receipt = match crate::receipt::decide(store, &operation, digest.as_str())? {
        crate::receipt::Decision::First => crate::receipt::first(&operation, digest.as_str(), 0),
        crate::receipt::Decision::Repeat(_) => {
            return Err(CaptureError::AttemptTerminalConflict.into())
        }
    };
    let mut batch = store.db.batch();
    codec::stage(
        store,
        &mut batch,
        &format!("{}/terminal", read::attempt_prefix(terminal.attempt())),
        &terminal,
        protocol,
    )?;
    if let Some(occurrence) = result {
        codec::stage(
            store,
            &mut batch,
            &format!("work/{}/result", terminal.attempt().work_id()),
            &WorkResult {
                version: protocol,
                occurrence: occurrence.clone(),
            },
            protocol,
        )?;
    }
    batch.insert(
        &store.receipts,
        operation.as_bytes(),
        serde_json::to_string(&receipt)
            .map_err(|source| StoreError::Json {
                detail: "terminal receipt".to_string(),
                source,
            })?
            .as_bytes(),
    );
    codec::commit(batch)?;
    Ok(terminal)
}
