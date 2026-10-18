use super::owners;
use super::research::{capture_for, CoachResearchStatus};
use anyhow::{Context, Result};
use census_crawl::school_sites::{PageEvidence, SiteOutcome, StaffContactHit};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct StaffContactCandidate {
    pub staff: StaffContactHit,
    pub source: PageEvidence,
}

pub(super) fn project(
    outcome: &SiteOutcome,
) -> Result<(Vec<StaffContactCandidate>, CoachResearchStatus)> {
    anyhow::ensure!(
        outcome.signals.staff_contacts.len() <= 16_384
            && outcome.signals.staff_pages.len() <= 14
            && outcome.pages.len() <= 14,
        "staff contact research exceeds its row or page budget"
    );
    let mut candidates = Vec::new();
    let mut bytes = 0usize;
    let mut incomplete = outcome.errors > 0
        || outcome.note.is_some()
        || !outcome
            .pages
            .iter()
            .any(|page| (200..300).contains(&page.status))
        || outcome
            .signals
            .staff_pages
            .iter()
            .any(|url| capture_for(outcome, url).is_none());
    for hit in &outcome.signals.staff_contacts {
        let Some(source) = capture_for(outcome, &hit.url)
            .filter(|_| outcome.signals.staff_pages.contains(&hit.url))
        else {
            incomplete = true;
            continue;
        };
        bytes = bytes
            .checked_add(owners::serialized_size(&(hit, source))?)
            .context("staff contact candidate byte counter overflow")?;
        anyhow::ensure!(
            bytes <= 4 * 1024 * 1024,
            "staff contact candidates exceed 4 MiB"
        );
        candidates.try_reserve(1)?;
        candidates.push(StaffContactCandidate {
            staff: hit.clone(),
            source: source.clone(),
        });
    }
    let status = if incomplete {
        CoachResearchStatus::Incomplete
    } else if outcome.signals.staff_pages.is_empty() {
        CoachResearchStatus::Unsupported
    } else if candidates.is_empty() {
        CoachResearchStatus::NoQualifiedCandidateInRecognizedStaffPages
    } else {
        CoachResearchStatus::UnverifiedCandidates
    };
    Ok((candidates, status))
}
