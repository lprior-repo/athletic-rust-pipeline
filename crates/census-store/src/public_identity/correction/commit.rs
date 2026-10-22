mod phases;
use super::{request, ReviewedIdentityCommitOutcomeV1, ReviewedIdentityRequestV1};
use crate::public_identity::{
    authority::graph,
    commit, graph_operation,
    graph_witness::{Decision, HeldGraphWitness, Payload},
    operation,
    paged_generation::VerifiedRawGraph,
    qualification::Ledger,
    PublicIdentityError, PublicIdentityWriterV1,
};
use crate::{Store, StoreResult, StoreRevision};
use census_domain::public_identity_history::{prepare_next, RegistryHead};
impl PublicIdentityWriterV1 {
    pub fn apply_reviewed_identity(
        &self,
        store: &Store,
        operation: &str,
        request: &ReviewedIdentityRequestV1,
        expected: StoreRevision<'_>,
        head: &RegistryHead,
    ) -> StoreResult<ReviewedIdentityCommitOutcomeV1> {
        crate::public_identity::commit::source_batch::refuse_child_operation(operation)?;
        let guard = store.lock_appends();
        expected.ensure_origin(store, operation)?;
        self.require_current_qualification()?;
        let ledger = self.verified(store)?;
        let digest = request::digest(&self.identity, operation, request)?;
        if let Some(outcome) = repeated(store, operation, &digest, &ledger)? {
            return Ok(outcome);
        }
        expected.ensure_current(store, operation)?;
        if head != ledger.history.projection().head() {
            return Err(PublicIdentityError::HeadMoved.into());
        }
        drop(guard);
        let capture = graph_operation::begin(self, store, operation, &digest, expected, head)?;
        let pending = phases::prepare(
            self, store, operation, request, &digest, head, capture, &ledger,
        )?;
        phases::finish(self, store, operation, &digest, expected, head, pending)
    }
}
fn repeated(
    store: &Store,
    operation: &str,
    digest: &str,
    ledger: &Ledger,
) -> StoreResult<Option<ReviewedIdentityCommitOutcomeV1>> {
    match operation::repeated(store, operation, digest, ledger)? {
        Some(operation::Replayed::Reviewed(outcome)) => Ok(Some(outcome)),
        Some(_) => Err(PublicIdentityError::OperationConflict.into()),
        None => Ok(None),
    }
}
