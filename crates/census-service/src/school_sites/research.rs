use super::owners;
use anyhow::{Context, Result};
use census_crawl::school_sites::{PageEvidence, SiteOutcome};
use census_domain::model::SchoolMailboxPurpose;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ResearchStatus {
    #[default]
    Unknown,
    UnverifiedCandidates,
    NoQualifiedCandidateInInspectedPages,
    Incomplete,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CoachResearchStatus {
    #[default]
    Unknown,
    Unsupported,
    UnverifiedCandidates,
    NoQualifiedCandidateInRecognizedStaffPages,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct SchoolMailboxCandidate {
    pub purpose: SchoolMailboxPurpose,
    pub mailbox: String,
    pub source: PageEvidence,
}

pub(super) fn project(
    outcome: &SiteOutcome,
) -> Result<(Vec<SchoolMailboxCandidate>, ResearchStatus)> {
    anyhow::ensure!(
        outcome.signals.school_mailboxes.len() <= 16_384 && outcome.pages.len() <= 14,
        "school mailbox research exceeds its row or page budget"
    );
    let mut candidates = Vec::new();
    let mut bytes = 0usize;
    let mut incomplete = outcome.errors > 0
        || outcome.note.is_some()
        || !outcome
            .pages
            .iter()
            .any(|page| (200..300).contains(&page.status));
    for hit in &outcome.signals.school_mailboxes {
        let Some(source) = capture_for(outcome, &hit.url) else {
            incomplete = true;
            continue;
        };
        bytes = bytes
            .checked_add(owners::serialized_size(&(hit, source))?)
            .context("school mailbox candidate byte counter overflow")?;
        anyhow::ensure!(
            bytes <= 4 * 1024 * 1024,
            "school mailbox candidates exceed 4 MiB"
        );
        candidates.try_reserve(1)?;
        candidates.push(SchoolMailboxCandidate {
            purpose: hit.purpose,
            mailbox: hit.email.clone(),
            source: source.clone(),
        });
    }
    let status = if incomplete {
        ResearchStatus::Incomplete
    } else if candidates.is_empty() {
        ResearchStatus::NoQualifiedCandidateInInspectedPages
    } else {
        ResearchStatus::UnverifiedCandidates
    };
    Ok((candidates, status))
}

pub(super) fn capture_for<'a>(outcome: &'a SiteOutcome, url: &str) -> Option<&'a PageEvidence> {
    let source = outcome
        .pages
        .iter()
        .find(|page| page.url == url && (200..300).contains(&page.status))?;
    (!outcome
        .pages
        .iter()
        .any(|page| page.url == url && page != source))
    .then_some(source)
}
