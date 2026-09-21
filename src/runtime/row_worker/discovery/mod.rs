mod fetch;
mod profiles;
mod queries;

use super::rankings::fold_rankings_lookup;
use super::support::{assess_and_publish, search_completeness};
use crate::domain::identity::{AthleteId, EvidenceDigest};
use crate::domain::name::CanonicalName;
use crate::model::SourceRecord;
use crate::runtime::Runtime;
use crate::search::SearchQuery;
use restate_sdk::prelude::*;
use std::collections::BTreeSet;
use std::sync::Arc;

pub(crate) use profiles::{execute_profiles, ProfileState};
pub(crate) use queries::execute_queries;

#[derive(Debug, Default)]
pub(crate) struct DiscoveryState {
    pub incomplete: bool,
    pub refs: Vec<EvidenceDigest>,
    pub issues: Vec<String>,
    pub candidate_ids: BTreeSet<AthleteId>,
    pub candidate_limit: bool,
}

impl DiscoveryState {
    pub(crate) fn complete(&self) -> bool {
        !self.incomplete && !self.candidate_limit && self.issues.is_empty()
    }
}

/// One row's discovery result: the folded candidate state plus the evidence published for it.
pub(crate) struct RowDiscovery {
    pub(crate) state: DiscoveryState,
    pub(crate) query_refs: Vec<EvidenceDigest>,
    pub(crate) summary: EvidenceDigest,
}

/// Run every planned query, fold the rankings lookup, and publish the discovery summary.
pub(crate) async fn discover_candidates(
    ctx: &ObjectContext<'_>,
    runtime: &Arc<Runtime>,
    job: &super::super::row_protocol::RowJob,
    source: &SourceRecord,
    queries: Vec<SearchQuery>,
) -> Result<RowDiscovery, HandlerError> {
    let mut state = execute_queries(ctx, runtime.clone(), &job.snapshot, queries).await?;
    let query_refs = std::mem::take(&mut state.refs);
    let source_name = CanonicalName::from_source(source).ok().flatten();
    let rankings = fold_rankings_lookup(ctx, runtime, job, source_name, &mut state).await?;
    let summary =
        super::support::publish_discovery_summary(ctx, runtime, job, &state, &query_refs, rankings)
            .await?;
    Ok(RowDiscovery {
        state,
        query_refs,
        summary,
    })
}

/// Acquire the candidate profiles and publish the assessment that scores them.
pub(crate) async fn assess_candidates(
    ctx: &ObjectContext<'_>,
    runtime: &Arc<Runtime>,
    source: &SourceRecord,
    snapshot: &EvidenceDigest,
    discovery: &RowDiscovery,
) -> Result<
    (ProfileState, (EvidenceDigest, crate::domain::decision::Assessment)),
    HandlerError,
> {
    let (profiles, _profile_refs) = execute_profiles(
        ctx,
        runtime.clone(),
        snapshot,
        source,
        &discovery.state.candidate_ids,
    )
    .await?;
    let search = search_completeness(&discovery.state, &profiles, &discovery.summary);
    Ok(assess_and_publish(ctx, runtime.clone(), source, profiles, search).await?)
}
