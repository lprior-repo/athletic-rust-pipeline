use super::*;
use crate::public_identity::{
    authority::graph::admission::demand::{self, GraphDemandV1},
    graph_reference::GraphAuthorityReferenceV1,
    local_adjudication::admission::{graph as admission, preparation},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SourceCohortDemandV1 {
    graph: GraphAuthorityReferenceV1,
    head: RegistryHead,
    requested_cohort: GradYear,
    request_digest: String,
    authority: GraphDemandV1,
    inspected_sequence: u64,
    finished_sequence: u64,
    scope: &'static str,
    unmeasured: [&'static str; 5],
}

impl PublicIdentityWriterV1 {
    pub fn measure_retained_source_cohort(
        &self,
        store: &Store,
        operation: &str,
        cohort: GradYear,
        expected: StoreRevision<'_>,
        head: &RegistryHead,
    ) -> StoreResult<SourceCohortDemandV1> {
        refuse_child_operation(operation)?;
        let request = Request::new(&self.identity, operation, cohort)?;
        let _guard = store.lock_appends();
        expected.ensure_origin(store, operation)?;
        self.require_current_qualification()?;
        expected.ensure_current(store, operation)?;
        let graph = retained_graph(self, store, &request, expected, head)?;
        let snapshot = store.snapshot_locked();
        let inspected_sequence = snapshot.sequence();
        let view = VerifiedRawGraph::verify(&snapshot, graph.capture(), graph.seal())?;
        let authority = demand::measure(&view)?;
        drop(view);
        drop(snapshot);
        drop(graph_operation::require_current(
            self,
            store,
            &request.operation,
            expected,
            head,
            &graph,
        )?);
        let finished_sequence = store.snapshot_locked().sequence();
        if finished_sequence != inspected_sequence {
            return Err(PublicIdentityError::Corrupt("diagnostic logical sequence changed").into());
        }
        Ok(SourceCohortDemandV1 {
            graph,
            head: head.clone(),
            requested_cohort: cohort,
            request_digest: request.digest,
            authority,
            inspected_sequence,
            finished_sequence,
            scope: "Current retained graph measurement only; requested cohort is not authenticated by the raw graph. Existing reservation audits and graph verification precede the diagnostic scan budgets. No cohort preparation or acceptance is performed.",
            unmeasured: ["grade_enrichment", "accepted_cohort", "history", "decision_and_witness_bytes", "working_set_and_atomic_batch"],
        })
    }
}

fn retained_graph(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    request: &Request,
    expected: StoreRevision<'_>,
    head: &RegistryHead,
) -> StoreResult<GraphAuthorityReferenceV1> {
    let raw = graph_operation::raw_operation(&writer.identity, &request.operation)?;
    let retained = admission::load_graph(store, &writer.identity, &raw)?.ok_or(
        PublicIdentityError::Corrupt("retained cohort graph is absent"),
    )?;
    if !retained.is_complete(store)? {
        return Err(PublicIdentityError::Corrupt("retained cohort graph is incomplete").into());
    }
    let graph = GraphAuthorityReferenceV1::new(retained.capture().clone(), retained.seal().clone());
    drop(graph_operation::require_current(
        writer,
        store,
        &request.operation,
        expected,
        head,
        &graph,
    )?);
    if let Some(prior) = preparation::load(store, &writer.identity, &request.operation)? {
        prior.require_request(
            &writer.identity,
            &request.operation,
            &request.digest,
            expected,
            head,
        )?;
        if prior.graph != graph {
            return Err(PublicIdentityError::OperationConflict.into());
        }
    }
    Ok(graph)
}

#[cfg(test)]
mod tests;
