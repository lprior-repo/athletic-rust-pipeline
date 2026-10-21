use super::ledger::{self, AdmissionReplay, Attempt, Completion, Identity};
use crate::restate_services::wire::{TeamsSourceOutcome, TeamsSourceRequest};
use crate::restate_services::JobError;
use census_domain::model::SourceAccessCondition;
use census_store::Store;

#[derive(Clone, Default)]
pub(super) enum ExecutionLedger {
    #[default]
    Original,
    OriginContinuation(ledger::origin_continuation::Authorization),
}

impl ExecutionLedger {
    pub(super) fn status(
        &self,
        store: &Store,
        key: &str,
        request: Option<&TeamsSourceRequest>,
    ) -> Result<Option<TeamsSourceOutcome>, JobError> {
        match self {
            Self::Original => ledger::status(store, key, request),
            Self::OriginContinuation(_) => ledger::origin_continuation::status(store, key),
        }
    }

    pub(super) fn baseline(&self, store: &Store, key: &str) -> Result<u8, JobError> {
        match self {
            Self::Original => ledger::baseline(store, key),
            Self::OriginContinuation(_) => ledger::origin_continuation::baseline(store, key),
        }
    }

    pub(super) fn begin_at(
        &self,
        store: &Store,
        key: &str,
        request: &TeamsSourceRequest,
        registered: &Identity,
        baseline: u8,
    ) -> Result<AdmissionReplay, JobError> {
        match self {
            Self::Original => ledger::begin_at(store, key, request, registered, baseline),
            Self::OriginContinuation(authorization) => {
                ledger::origin_continuation::begin_at(store, key, request, authorization, baseline)
            }
        }
    }

    pub(super) fn finish(
        &self,
        store: &Store,
        key: &str,
        attempt: u8,
        result: Attempt,
        conditions: &[SourceAccessCondition],
    ) -> Result<Completion, JobError> {
        match self {
            Self::Original => ledger::finish(store, key, attempt, result, conditions),
            Self::OriginContinuation(_) => {
                ledger::origin_continuation::finish(store, key, attempt, result, conditions)
            }
        }
    }
}
