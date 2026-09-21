use super::discovery::DiscoveryState;
use super::super::row_protocol::{RankingDiscoveryEvidence, RowJob};
use super::{TerminalError, MAX_CANDIDATES};
use crate::{domain::name::CanonicalName, runtime::Runtime, store::rankings::RankingLookup};
use restate_sdk::prelude::*;
use std::sync::Arc;

/// Perform a rankings lookup by canonical name.
/// Requires the store seal to match the bound snapshot.
/// Returns the lookup and whether the lookup was incomplete (None name).
pub(crate) async fn perform_rankings_lookup(
    ctx: &ObjectContext<'_>,
    runtime: Arc<Runtime>,
    collection: crate::domain::identity::EvidenceDigest,
    bound_snapshot: crate::domain::identity::EvidenceDigest,
    source_name: Option<CanonicalName>,
) -> std::result::Result<(RankingLookup, bool), TerminalError> {
    let store = runtime.store.clone();
    ctx.run(|| async move {
        runtime
            .blocking(move || {
                let sealed = store.ranking_snapshot(&collection)?;
                let expected = sealed.ok_or_else(|| {
                    anyhow::anyhow!(
                        "collection has no sealed snapshot, cannot perform rankings lookup"
                    )
                })?;
                if expected != bound_snapshot {
                    anyhow::bail!(
                        "collection seal {:?} does not match bound snapshot {:?}",
                        expected,
                        bound_snapshot
                    );
                }
                let (lookup, incomplete) = match source_name {
                    Some(name) => (
                        store
                            .ranking_name_refs(&collection, &name, MAX_CANDIDATES)
                            .map_err(|e| anyhow::anyhow!("rankings lookup failed: {e}"))?,
                        false,
                    ),
                    None => (
                        RankingLookup {
                            records: Vec::new(),
                            truncated: false,
                        },
                        true,
                    ),
                };
                Ok::<(RankingLookup, bool), anyhow::Error>((lookup, incomplete))
            })
            .await
            .map_err(|e| anyhow::anyhow!("rankings lookup blocking failed: {e}"))
            .map(Json)
            .map_err(|e| TerminalError::new(e.to_string()).into())
    })
    .name("rankings-lookup")
    .await
    .map(|Json(tuple)| tuple)
}

/// Fold a rankings lookup into the row's discovery state.
///
/// Returns the evidence to publish when the row was bound to a collection; `None` when the job
/// carries no rankings reference at all.
pub(crate) async fn fold_rankings_lookup(
    ctx: &ObjectContext<'_>,
    runtime: &Arc<Runtime>,
    job: &RowJob,
    source_name: Option<CanonicalName>,
    discovery: &mut DiscoveryState,
) -> Result<Option<RankingDiscoveryEvidence>, HandlerError> {
    let Some(ranking_ref) = &job.rankings else {
        return Ok(None);
    };
    let collection = ranking_ref.collection.clone();
    let bound_snapshot = ranking_ref.snapshot.clone();
    let (lookup, lookup_incomplete) = perform_rankings_lookup(
        ctx,
        runtime.clone(),
        collection,
        bound_snapshot,
        source_name.clone(),
    )
    .await?;
    if lookup_incomplete || lookup.truncated {
        discovery.incomplete = true;
    }
    let mut candidate_limit = false;
    let records = &lookup.records;
    for ref_entry in records {
        if discovery.candidate_ids.contains(&ref_entry.athlete_id) {
            continue;
        }
        if discovery.candidate_ids.len() == MAX_CANDIDATES {
            candidate_limit = true;
            break;
        }
        discovery.candidate_ids.insert(ref_entry.athlete_id);
    }
    if candidate_limit {
        discovery.candidate_limit = true;
    }
    Ok(Some(RankingDiscoveryEvidence {
        canonical_name: source_name,
        collection: ranking_ref.clone(),
        lookup,
    }))
}
