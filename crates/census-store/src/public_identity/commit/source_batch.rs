use super::*;
use crate::public_identity::{
    operation::Replayed, paged_generation::AuthorityTableV2, qualification::Ledger,
};
use census_domain::model::{AcceptedAthleteIdentity, CanonicalAthlete, GradYear, SourceIdentity};
use census_domain::public_identity_history::HistoryProjection;
use std::collections::BTreeSet;

const PREFIX: &str = "source-cohort-v1:";

mod audit;
mod persist;
mod prepare;
mod replay;
#[cfg(test)]
mod tests;
pub(in crate::public_identity) use audit::audit;

struct Request {
    operation: String,
    digest: String,
    prefix: String,
    cohort: GradYear,
}
struct Item {
    accepted: AcceptedAthleteIdentity,
    record: HistoryRecord,
    witness: HeldGraphWitness,
}
struct Pending {
    capture: graph_operation::Capture,
    items: Vec<Item>,
    head: RegistryHead,
    history: Vec<u8>,
}

impl Request {
    fn new(identity: &StoreIdentity, operation: &str, cohort: GradYear) -> StoreResult<Self> {
        graph_operation::raw_operation(identity, operation)?;
        let digest = serialized_digest(&("source-cohort-v1", identity, operation, cohort))?;
        Ok(Self {
            operation: format!(
                "source-cohort-v1:{}",
                crate::journal::object_digest(operation.as_bytes())
            ),
            prefix: format!("source-cohort-v1:{digest}:"),
            digest,
            cohort,
        })
    }
    fn child(&self, candidate: &AthleteCandidateId) -> StoreResult<String> {
        Ok(format!("{}{}", self.prefix, serialized_digest(candidate)?))
    }
}

impl PublicIdentityWriterV1 {
    pub fn establish_source_bound_cohort(
        &self,
        store: &Store,
        operation: &str,
        cohort: GradYear,
        expected: StoreRevision<'_>,
        head: &RegistryHead,
    ) -> StoreResult<Vec<IdentityCommitOutcomeV1>> {
        refuse_child_operation(operation)?;
        let request = Request::new(&self.identity, operation, cohort)?;
        let guard = store.lock_appends();
        expected.ensure_origin(store, operation)?;
        self.require_current_qualification()?;
        let ledger = self.verified(store)?;
        if let Some(outcomes) = replay::replay(self, store, &request, &ledger)? {
            return Ok(outcomes);
        }
        expected.ensure_current(store, operation)?;
        if head != ledger.history.projection().head() {
            return Err(PublicIdentityError::HeadMoved.into());
        }
        drop(guard);
        let capture = match graph_operation::begin(
            self,
            store,
            &request.operation,
            &request.digest,
            expected,
            head,
        ) {
            Ok(capture) => capture,
            Err(error) => {
                let _guard = store.lock_appends();
                let ledger = self.verified(store)?;
                if let Some(outcomes) = replay::replay(self, store, &request, &ledger)? {
                    return Ok(outcomes);
                }
                return Err(error);
            }
        };
        if capture.reserved.is_some() {
            return Err(PublicIdentityError::OperationConflict.into());
        }
        let pending = prepare::prepare(self, store, &request, ledger, capture)?;
        persist::finish(self, store, &request, expected, head, pending)
    }
}

fn outcome(
    held: &HeldGraphWitness,
    record: &HistoryRecord,
    replayed: bool,
) -> StoreResult<IdentityCommitOutcomeV1> {
    match held.outcome(record, replayed)? {
        Replayed::Source(value) => Ok(value),
        _ => Err(PublicIdentityError::OperationConflict.into()),
    }
}

fn push<T>(values: &mut Vec<T>, value: T) -> StoreResult<()> {
    values.try_reserve(1).map_err(|error| StoreError::Refused {
        detail: format!("source cohort allocation: {error}"),
    })?;
    values.push(value);
    Ok(())
}

pub(in crate::public_identity) fn is_reserved_operation(value: &[u8]) -> bool {
    value.starts_with(PREFIX.as_bytes())
}

pub(in crate::public_identity) fn refuse_child_operation(operation: &str) -> StoreResult<()> {
    if is_reserved_operation(operation.as_bytes()) {
        return Err(PublicIdentityError::OperationConflict.into());
    }
    Ok(())
}
