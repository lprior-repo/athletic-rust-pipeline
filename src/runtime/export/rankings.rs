use super::index::load_json;
use crate::{
    domain::identity::EvidenceDigest,
    runtime::{
        identity::fingerprint,
        rankings_collection::{collection_fingerprint, CollectionFinalSnapshot},
        run_protocol::{RunProgress, SourceSnapshot},
    },
    store::ArtifactStore,
};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RankingsExportCoverage {
    pub collection: EvidenceDigest,
    pub source_snapshot: EvidenceDigest,
    pub scope_fingerprint: EvidenceDigest,
    pub sealed_snapshot: Option<EvidenceDigest>,
    pub observed_queries_complete: bool,
    pub relay_membership_complete: bool,
    pub observed_queries: Option<u64>,
    pub completed_queries: Option<u64>,
    pub absent_families: Vec<String>,
    pub unique_athletes: Option<u64>,
    pub unresolved_roster_results: Option<u64>,
}

pub(super) fn coverage(
    store: &ArtifactStore,
    progress: &RunProgress,
) -> Result<Option<RankingsExportCoverage>> {
    let source: SourceSnapshot = load_json(store, &progress.request.snapshot)?;
    let Some(scope) = source.rankings else {
        if progress.collection_ref.is_some() {
            bail!("unscoped run contains a ranking collection");
        }
        return Ok(None);
    };
    scope.validate()?;
    let collection = collection_fingerprint(&scope.revision, &progress.request.snapshot)?;
    let mut result = RankingsExportCoverage {
        collection: collection.clone(),
        source_snapshot: progress.request.snapshot.clone(),
        scope_fingerprint: fingerprint(&scope)?,
        sealed_snapshot: None,
        observed_queries_complete: false,
        relay_membership_complete: false,
        observed_queries: None,
        completed_queries: None,
        absent_families: Vec::new(),
        unique_athletes: None,
        unresolved_roster_results: None,
    };
    let Some(bound) = &progress.collection_ref else {
        return Ok(Some(result));
    };
    if bound.collection != collection
        || store.ranking_snapshot(&collection)?.as_ref() != Some(&bound.snapshot)
    {
        bail!("export collection differs from the source snapshot or immutable seal");
    }
    let final_snapshot: CollectionFinalSnapshot = load_json(store, &bound.snapshot)?;
    if final_snapshot.collection != collection
        || final_snapshot.source_snapshot != progress.request.snapshot
        || fingerprint(&final_snapshot.scope)? != result.scope_fingerprint
    {
        bail!("export final snapshot has a foreign source or scope");
    }
    let missing = final_snapshot
        .event_heads
        .iter()
        .try_fold(0_u64, |count, event| {
            count
                .checked_add(
                    store
                        .ranking_event_stats(&collection, &event.event_short)?
                        .unresolved_roster_results,
                )
                .context("relay roster count overflow")
        })?;
    result.sealed_snapshot = Some(bound.snapshot.clone());
    result.observed_queries_complete = final_snapshot.coverage.total_requested > 0
        && final_snapshot.coverage.completed == final_snapshot.coverage.total_requested
        && final_snapshot.event_heads.len() as u64 == final_snapshot.coverage.total_requested
        && final_snapshot
            .event_heads
            .iter()
            .all(|event| event.terminal && event.head_checkpoint.is_some());
    result.relay_membership_complete = result.observed_queries_complete && missing == 0;
    result.observed_queries = Some(final_snapshot.coverage.total_requested);
    result.completed_queries = Some(final_snapshot.coverage.completed);
    result.absent_families = final_snapshot.coverage.absent_families;
    result.unique_athletes = Some(final_snapshot.unique_athletes);
    result.unresolved_roster_results = Some(missing);
    Ok(Some(result))
}
