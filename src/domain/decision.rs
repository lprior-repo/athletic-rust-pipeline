use crate::domain::{
    candidate::{CandidateCoverage, CandidateEvidence},
    evidence::ProfileEvidence,
    identity::{AthleteId, EvidenceDigest},
};
use crate::model::SourceRecord;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

mod matching;

const MAX_SOURCE_TEXT_BYTES: usize = 4_096;
const MAX_PROFILES: usize = 4_096;
const MAX_SEARCH_REASONS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchCompleteness {
    Complete { evidence: EvidenceDigest },
    Incomplete { reasons: Vec<String> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Decision {
    DeterministicAccepted,
    IdentityReview,
    EvidenceReview,
    CompleteSearchNoMatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateReason {
    athlete_id: AthleteId,
    exact_name: bool,
    matching_school: bool,
    location_corroborated: bool,
    hard_eligible: bool,
    participation_confirmed: bool,
    evidence_conflict: bool,
    evidence_strength: u8,
    reasons: Vec<String>,
    evidence: Vec<EvidenceDigest>,
    mailing_location_matches: bool,
    coverage: CandidateCoverage,
}

impl CandidateReason {
    #[must_use]
    pub fn athlete_id(&self) -> AthleteId {
        self.athlete_id
    }
    #[must_use]
    pub fn exact_name(&self) -> bool {
        self.exact_name
    }
    #[must_use]
    pub fn matching_school(&self) -> bool {
        self.matching_school
    }
    #[must_use]
    pub fn location_corroborated(&self) -> bool {
        self.location_corroborated
    }
    #[must_use]
    pub fn hard_eligible(&self) -> bool {
        self.hard_eligible
    }
    #[must_use]
    pub fn evidence_strength(&self) -> u8 {
        self.evidence_strength
    }
    #[must_use]
    pub fn reasons(&self) -> &[String] {
        &self.reasons
    }
    #[must_use]
    pub fn evidence(&self) -> &[EvidenceDigest] {
        &self.evidence
    }
    #[must_use]
    pub fn participation_confirmed(&self) -> bool {
        self.participation_confirmed
    }
    #[must_use]
    pub fn evidence_conflict(&self) -> bool {
        self.evidence_conflict
    }
    #[must_use]
    pub fn coverage(&self) -> CandidateCoverage {
        self.coverage
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct VerifiedMatch {
    athlete_id: AthleteId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Assessment {
    decision: Decision,
    candidates: Vec<CandidateReason>,
    search: SearchCompleteness,
    verified: Option<VerifiedMatch>,
}

impl Assessment {
    #[must_use]
    pub fn decision(&self) -> Decision {
        self.decision
    }
    #[must_use]
    pub fn candidates(&self) -> &[CandidateReason] {
        &self.candidates
    }
    #[must_use]
    pub fn search(&self) -> &SearchCompleteness {
        &self.search
    }
    #[must_use]
    pub fn accepted_athlete_id(&self) -> Option<AthleteId> {
        self.verified.as_ref().map(|matched| matched.athlete_id)
    }
    #[must_use]
    pub fn is_deterministic_acceptance(&self) -> bool {
        matches!(self.decision, Decision::DeterministicAccepted)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReviewChoice {
    Select(AthleteId),
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FinalDecision {
    accepted: Option<VerifiedMatch>,
}

impl FinalDecision {
    fn accepted(athlete_id: AthleteId) -> Self {
        Self {
            accepted: Some(VerifiedMatch { athlete_id }),
        }
    }

    #[must_use]
    pub fn accepted_athlete_id(&self) -> Option<AthleteId> {
        self.accepted.as_ref().map(|matched| matched.athlete_id)
    }
}

struct CandidateGroup<'a> {
    profiles: Vec<&'a ProfileEvidence>,
    coverage: CandidateCoverage,
    inconsistent: bool,
    exclusion_documents: Vec<EvidenceDigest>,
}

pub fn assess<'a, I>(
    record: &SourceRecord,
    evidence: I,
    search: SearchCompleteness,
) -> Result<Assessment>
where
    I: IntoIterator<Item = CandidateEvidence<'a>>,
{
    let entries = evidence
        .into_iter()
        .take(MAX_PROFILES + 1)
        .collect::<Vec<_>>();
    validate_input(record, &entries, &search)?;
    let groups = group_evidence(&entries);
    let source = matching::source_identity(record);
    let candidates = groups
        .iter()
        .map(|(id, group)| {
            let coverage = if group.inconsistent {
                CandidateCoverage::Incomplete
            } else {
                group.coverage
            };
            matching::candidate_reason(
                *id,
                &group.profiles,
                &source,
                coverage,
                &group.exclusion_documents,
            )
        })
        .collect::<Vec<_>>();
    let incomplete = groups
        .values()
        .any(|group| group.inconsistent || matches!(group.coverage, CandidateCoverage::Incomplete));
    let (decision, verified) = choose_decision(&source, &candidates, &search, incomplete);
    Ok(Assessment {
        decision,
        candidates,
        search,
        verified,
    })
}

fn group_evidence<'a>(
    entries: &[CandidateEvidence<'a>],
) -> std::collections::BTreeMap<AthleteId, CandidateGroup<'a>> {
    entries
        .iter()
        .fold(std::collections::BTreeMap::new(), |mut groups, entry| {
            let id = (*entry).athlete_id();
            let coverage = (*entry).coverage();
            let group = groups.entry(id).or_insert_with(|| CandidateGroup {
                profiles: Vec::new(),
                coverage,
                inconsistent: false,
                exclusion_documents: Vec::new(),
            });
            if group.coverage != coverage {
                group.inconsistent = true;
            }
            group.profiles.extend((*entry).profiles().iter());
            if let Some(exclusion) = (*entry).exclusion() {
                group
                    .exclusion_documents
                    .extend(exclusion.documents().iter().cloned());
            }
            groups
        })
}

pub fn apply_review(assessment: &Assessment, choice: ReviewChoice) -> Result<FinalDecision> {
    match choice {
        ReviewChoice::Unresolved => Ok(assessment
            .accepted_athlete_id()
            .map_or(FinalDecision { accepted: None }, FinalDecision::accepted)),
        ReviewChoice::Select(athlete_id) => apply_selection(assessment, athlete_id),
    }
}

fn apply_selection(assessment: &Assessment, athlete_id: AthleteId) -> Result<FinalDecision> {
    if !matches!(assessment.search, SearchCompleteness::Complete { .. }) {
        bail!("selection cannot override incomplete discovery");
    }
    if matches!(
        assessment.decision,
        Decision::EvidenceReview | Decision::CompleteSearchNoMatch
    ) {
        bail!("selection cannot override insufficient or contradictory evidence")
    }
    let candidate = assessment
        .candidates
        .iter()
        .find(|candidate| candidate.athlete_id == athlete_id && candidate.hard_eligible)
        .ok_or_else(|| anyhow::anyhow!("selected athlete is not an eligible supplied candidate"))?;
    if matches!(assessment.decision, Decision::IdentityReview)
        && !selection_distinguishes(assessment, candidate)
    {
        bail!("supplied evidence does not distinguish the selected candidate")
    }
    Ok(FinalDecision::accepted(athlete_id))
}

fn selection_distinguishes(assessment: &Assessment, selected: &CandidateReason) -> bool {
    assessment
        .candidates
        .iter()
        .filter(|candidate| candidate.hard_eligible)
        .filter(|candidate| {
            candidate.exact_name == selected.exact_name
                && candidate.matching_school == selected.matching_school
                && candidate.location_corroborated == selected.location_corroborated
                && candidate.mailing_location_matches == selected.mailing_location_matches
        })
        .count()
        == 1
}

fn validate_input(
    record: &SourceRecord,
    entries: &[CandidateEvidence<'_>],
    search: &SearchCompleteness,
) -> Result<()> {
    if entries.len() > MAX_PROFILES {
        bail!("candidate evidence exceeds {MAX_PROFILES} entries")
    }
    if entries
        .iter()
        .flat_map(|entry| (*entry).profiles().iter())
        .take(MAX_PROFILES * 2 + 1)
        .count()
        > MAX_PROFILES * 2
    {
        bail!("profile fragments exceed twice the candidate bound")
    }
    let source_name = crate::domain::name::CanonicalName::from_source(record)?;
    entries.iter().try_for_each(|entry| {
        let athlete_id = (*entry).athlete_id();
        if (*entry)
            .profiles()
            .iter()
            .any(|profile| profile.athlete_id != athlete_id)
        {
            bail!("candidate profile evidence has a mismatched athlete ID")
        }
        if (*entry)
            .exclusion()
            .is_some_and(|exclusion| source_name.as_ref() != Some(exclusion.source_name()))
        {
            bail!("name exclusion belongs to another source-name context")
        }
        Ok::<(), anyhow::Error>(())
    })?;
    const IDENTITY_FIELDS: [&str; 5] = [
        "Person First",
        "Person Last",
        "Schools Name",
        "Address Mailing / Permanent City",
        "Address Mailing / Permanent Region",
    ];
    IDENTITY_FIELDS
        .iter()
        .filter_map(|field| record.fields.get(*field))
        .try_for_each(|value| validate_text(value, "identity field"))?;
    if let SearchCompleteness::Incomplete { reasons } = search {
        if reasons.len() > MAX_SEARCH_REASONS {
            bail!("search incompleteness has too many reasons")
        }
        reasons
            .iter()
            .try_for_each(|reason| validate_text(reason, "search reason"))?;
    }
    Ok(())
}

fn validate_text(value: &str, field: &str) -> Result<()> {
    if value.len() > MAX_SOURCE_TEXT_BYTES {
        bail!("{field} exceeds {MAX_SOURCE_TEXT_BYTES} bytes")
    }
    if value.chars().any(char::is_control) {
        bail!("{field} contains a control character")
    }
    Ok(())
}

fn choose_decision(
    source: &matching::SourceIdentity,
    candidates: &[CandidateReason],
    search: &SearchCompleteness,
    incomplete: bool,
) -> (Decision, Option<VerifiedMatch>) {
    if source.name.is_none()
        || source.school.is_none()
        || !matches!(search, SearchCompleteness::Complete { .. })
        || incomplete
    {
        return (Decision::EvidenceReview, None);
    }
    let uncertain = candidates.iter().any(|candidate| {
        candidate.exact_name && candidate.matching_school && !candidate.hard_eligible
    });
    if uncertain {
        return (Decision::EvidenceReview, None);
    }
    let mut eligible = candidates
        .iter()
        .filter(|candidate| candidate.hard_eligible);
    match (eligible.next(), eligible.next()) {
        (Some(candidate), None) => (
            Decision::DeterministicAccepted,
            Some(VerifiedMatch {
                athlete_id: candidate.athlete_id,
            }),
        ),
        (Some(_), Some(_)) => (Decision::IdentityReview, None),
        (None, None) => (Decision::CompleteSearchNoMatch, None),
        (None, Some(_)) => (Decision::EvidenceReview, None),
    }
}

#[cfg(test)]
mod tests;
