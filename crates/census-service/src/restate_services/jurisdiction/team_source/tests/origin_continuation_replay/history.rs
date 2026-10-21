use super::{fixture::TestResult, key, ledger, request};
use crate::restate_services::wire::{
    OriginHeldContinuationRequest, TeamsAttemptProgress, TeamsSourceOutcome, TeamsSourceRequest,
};
use crate::restate_services::JobError;
use census_domain::model::serialized_digest;
use census_store::Store;

pub(super) struct Seed {
    pub(super) original: TeamsSourceRequest,
    pub(super) operation: String,
    pub(super) continuation: OriginHeldContinuationRequest,
    pub(super) original_output: Vec<u8>,
    original_phase: Vec<(String, serde_json::Value)>,
}

impl Seed {
    pub(super) fn new(store: &Store) -> TestResult<Self> {
        let mut original = request()?;
        original.source = "milesplit".into();
        let operation = key(&original);
        let identity = ledger::register(store, &operation, &original)?;
        check!(matches!(
            ledger::begin_at(store, &operation, &original, &identity, 0)?,
            ledger::AdmissionReplay::Reserved { attempt: 1, .. }
        ));
        let holder = serde_json::json!({"command": "controlled foreign owner",
            "origin": "https://oh.milesplit.com", "pid": 12345,
            "started_at": "2026-10-21T00:00:00Z"});
        let message = format!("origin https://oh.milesplit.com is held by another census-service process (holder: {holder})");
        let ledger::Completion::Settled(outcome) = ledger::finish(
            store,
            &operation,
            1,
            ledger::Attempt::Failed(JobError::Terminal { message }),
            &[],
        )?
        else {
            return Err("original ownership failure did not settle".into());
        };
        let continuation = OriginHeldContinuationRequest {
            original: original.clone(),
            original_outcome_digest: serialized_digest(&outcome)?,
        };
        Ok(Self {
            original,
            operation,
            continuation,
            original_output: serde_json::to_vec(&outcome)?,
            original_phase: phase(store, ledger::PHASE)?,
        })
    }

    pub(super) fn assert_original(&self, store: &Store) -> TestResult {
        check!(eq; phase(store, ledger::PHASE)?, self.original_phase);
        let outcome = ledger::status(store, &self.operation, Some(&self.original))?
            .ok_or("original ownership outcome missing")?;
        check!(eq; serde_json::to_vec(&outcome)?, self.original_output);
        check!(eq; ledger::baseline(store, &self.operation)?, 1);
        check!(!store.journal_contains(
            ledger::PHASE,
            &format!("{}/attempt/2/reserved", self.operation)
        )?);
        Ok(())
    }

    pub(super) fn assert_continued(&self, store: &Store) -> TestResult<TeamsSourceOutcome> {
        let phase = ledger::origin_continuation::PHASE;
        let outcome = ledger::origin_continuation::status(store, &self.operation)?
            .ok_or("continued outcome missing")?;
        let TeamsSourceOutcome::Terminal { message, progress } = &outcome else {
            return Err("offline continuation did not settle terminally".into());
        };
        check!(message.contains("offline and not cached:"));
        check!(message.contains("oh.milesplit.com"));
        check!(eq; progress.len(), 2);
        let original: TeamsSourceOutcome = serde_json::from_slice(&self.original_output)?;
        let TeamsSourceOutcome::Terminal {
            progress: original_progress,
            ..
        } = original
        else {
            return Err("seed ownership outcome was not terminal".into());
        };
        check!(eq; serde_json::to_value(progress.first())?, serde_json::to_value(original_progress.first())?);
        check!(
            matches!(progress.last(), Some(TeamsAttemptProgress::Terminal {
            attempt: 2, outcome: None, message: last,
        }) if last == message)
        );
        check!(eq; ledger::origin_continuation::baseline(store, &self.operation)?, 2);
        for suffix in [
            "authorization",
            "attempt/2/reserved",
            "attempt/2/outcome",
            "settled",
        ] {
            check!(store.journal_contains(phase, &format!("{}/{suffix}", self.operation))?);
        }
        for phase in [phase, ledger::PHASE] {
            check!(!store
                .journal_contains(phase, &format!("{}/attempt/3/reserved", self.operation))?);
        }
        Ok(outcome)
    }
}

pub(super) fn phase(store: &Store, phase: &str) -> TestResult<Vec<(String, serde_json::Value)>> {
    let mut keys: Vec<_> = store.journal_keys(phase)?.into_iter().collect();
    keys.sort();
    let snapshot = store.snapshot();
    keys.into_iter()
        .map(|key| {
            let value = snapshot
                .journal_payload(phase, &key)?
                .ok_or("journal row disappeared")?;
            Ok((key, value))
        })
        .collect()
}
