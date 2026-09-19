mod records;
mod verification;

use crate::{
    domain::name::CanonicalName,
    model::SourceRecord,
    runtime::{row_protocol::DiscoverySummary, row_worker::MAX_CANDIDATES},
    store::{ArtifactStore, RankingLookup},
};
use anyhow::{bail, Result};

/// An empty exact lookup is valid; an unusable source name or truncation is incomplete.
pub(super) fn verify(
    source: &SourceRecord,
    discovery: &DiscoverySummary,
    store: &ArtifactStore,
) -> Result<bool> {
    let (bound, evidence) = match (&discovery.job.rankings, &discovery.rankings) {
        (None, None) => return Ok(true),
        (Some(bound), Some(evidence)) if *bound == evidence.collection => (bound, evidence),
        _ => bail!("ranking witness differs from row job binding"),
    };
    let name = CanonicalName::from_source(source).ok().flatten();
    if evidence.canonical_name != name {
        bail!("ranking lookup name differs from source identity");
    }
    let (snapshot, plan, origin) = verification::collection(bound, &discovery.job.snapshot, store)?;
    let expected = match &name {
        Some(name) => store.ranking_name_refs(&bound.collection, name, MAX_CANDIDATES)?,
        None => RankingLookup {
            records: Vec::new(),
            truncated: false,
        },
    };
    if serde_json::to_value(&expected)? != serde_json::to_value(&evidence.lookup)? {
        bail!("ranking lookup order, records, or truncation differ from immutable index");
    }
    if let Some(name) = &name {
        records::verify(&expected.records, name, &snapshot, &plan, &origin, store)?;
    }
    Ok(name.is_some() && !expected.truncated)
}
