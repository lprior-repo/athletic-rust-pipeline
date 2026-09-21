//! Pagination replay for retained query evidence.

use super::verify_page;
use crate::{runtime::acquisition::QueryEvidence, search::SearchProgress, store::ArtifactStore};
use anyhow::{bail, Context, Result};
use url::Url;

pub(super) fn replay_pages(
    evidence: &QueryEvidence,
    origin: &Url,
    store: &ArtifactStore,
) -> Result<(
    SearchProgress,
    Vec<crate::domain::identity::AthleteId>,
    Option<u32>,
)> {
    // QueryWorker shares results by case-insensitive text and sport, not planning
    // stage. Verify the actual cached request after binding its identity to the plan.
    let mut progress = SearchProgress::new(evidence.query.clone());
    let mut candidate_ids = Vec::new();
    let mut reconciliation_start = None;
    evidence
        .pages
        .iter()
        .enumerate()
        .try_for_each(|(index, page)| {
            let expected_start = progress
                .next_offset()
                .context("query evidence contains a page after pagination completed")?;
            let parsed = verify_page(&evidence.query, page, expected_start, origin, store)?;
            candidate_ids.extend(parsed.results().iter().map(|candidate| candidate.id()));
            if reconciliation_start.is_some() {
                bail!("query evidence retains a page after pagination failure");
            }
            match progress.consume(parsed) {
                Ok(()) => Ok(()),
                Err(_) => {
                    reconciliation_start = Some(expected_start);
                    if index.checked_add(1) != Some(evidence.pages.len()) {
                        bail!("query evidence retains a page after reconciliation failure");
                    }
                    Ok::<(), anyhow::Error>(())
                }
            }
        })?;
    Ok((progress, candidate_ids, reconciliation_start))
}
