//! Assessment pipeline: group the supplied candidate evidence by athlete, fold
//! each group into reasons, and pick the decision that evidence supports.

use super::matching;
use super::validation::validate_input;
use super::MAX_PROFILES;
use super::{Assessment, CandidateReason, Decision, SearchCompleteness, VerifiedMatch};
use crate::domain::candidate::{CandidateCoverage, CandidateEvidence};
use crate::domain::evidence::ProfileEvidence;
use crate::domain::identity::{AthleteId, EvidenceDigest};
use crate::model::SourceRecord;
use anyhow::Result;

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
