use super::{MAX_CANDIDATES, TerminalError};
use crate::{
    domain::name::CanonicalName,
    runtime::Runtime,
    store::rankings::RankingLookup,
};
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
                    anyhow::anyhow!("collection has no sealed snapshot, cannot perform rankings lookup")
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
