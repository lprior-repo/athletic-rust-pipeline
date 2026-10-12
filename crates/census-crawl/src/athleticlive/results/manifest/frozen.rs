use super::{admission, parse_manifest, CaptureEntry};
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrozenManifest {
    pub body: String,
    pub digest: String,
    pub jurisdictions: Vec<UsJurisdiction>,
}

#[derive(Serialize, Deserialize)]
struct Envelope {
    meets: Vec<CaptureEntry>,
}

pub fn freeze_manifest(ctx: &AdapterContext<'_>, input: &str) -> CrawlResult<FrozenManifest> {
    let body = admission::read(input)?;
    let options = parse_manifest(&body, &ctx.observed_on)?;
    let mut envelope: Envelope = serde_json::from_str(&body).map_err(schema)?;
    let mut jurisdictions: Vec<_> = options
        .iter()
        .filter_map(|option| option.meet.as_ref().map(|meet| meet.state))
        .collect();
    jurisdictions.sort_unstable();
    jurisdictions.dedup();
    for entry in &mut envelope.meets {
        freeze_entry(ctx, entry)?;
    }
    let body = serde_json::to_string(&envelope).map_err(schema)?;
    admission::limit(
        "LIVE frozen manifest bytes",
        body.len(),
        admission::MAX_BYTES,
    )?;
    Ok(FrozenManifest {
        digest: crate::net::cache::content_digest(body.as_bytes()),
        body,
        jurisdictions,
    })
}

fn freeze_entry(ctx: &AdapterContext<'_>, entry: &mut CaptureEntry) -> CrawlResult<()> {
    if entry.documents.is_empty() && entry.summary.is_none() && entry.standings.is_empty() {
        return Err(CrawlError::Invariant {
            detail: format!(
                "manifest meet {} has no retained summary, event document or standings capture",
                entry.athleticlive_meet_id
            ),
        });
    }
    let mut metadata = std::collections::BTreeMap::new();
    for path in entry
        .documents
        .iter_mut()
        .chain(entry.summary.iter_mut())
        .chain(entry.standings.iter_mut().map(|row| &mut row.path))
    {
        let meta = entry.captures.get(path).ok_or_else(|| CrawlError::Schema {
            url: path.clone(),
            detail: "capture has no physical producer metadata".into(),
        })?;
        let captured = crate::athleticlive::capture::freeze(ctx, path, Some(meta), 1024 * 1024)?;
        *path =
            captured
                .path
                .into_os_string()
                .into_string()
                .map_err(|_| CrawlError::Invariant {
                    detail: "immutable capture path is not UTF-8".into(),
                })?;
        metadata.insert(path.clone(), meta.clone());
    }
    entry.captures = metadata;
    Ok(())
}

fn schema(error: serde_json::Error) -> CrawlError {
    CrawlError::Invariant {
        detail: format!("LIVE frozen manifest: {error}"),
    }
}
