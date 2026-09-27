#[cfg(test)]
use super::CaseEvidence;
use super::{
    identity_verdict_digest, AppliedAthleteIdentity, AppliedIdentityKind, AthleteCandidateId,
    AthleteIdentityIndex, IdentityError, ReviewCase, ReviewState, ReviewVerdictRecord,
    ATHLETE_IDENTITY_FAMILY, ATHLETE_IDENTITY_POLICY,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum IdentityDecisionIssue {
    WrongPolicy,
    InvalidMembership,
    StaleMemberEvidence,
    RetainedConflict,
    InvalidCanonicalTarget,
    CompetingSourceClaims,
    MissingResolvedCase,
    StaleCaseEvidence,
    MissingAcceptedVerdict,
    MissingPositiveIdentityEvidence,
    UnresolvedReview,
    ConflictingApplications,
}

pub(super) struct ReviewBindings<'a> {
    pub(super) cases: BTreeMap<&'a str, &'a ReviewCase>,
    pub(super) verdicts: BTreeMap<&'a str, &'a ReviewVerdictRecord>,
    pub(super) reviewed_subjects: BTreeSet<&'a str>,
}

impl<'a> ReviewBindings<'a> {
    pub(super) fn new(
        index: &AthleteIdentityIndex,
        cases: &'a [ReviewCase],
        verdicts: &'a [ReviewVerdictRecord],
    ) -> Result<Self, IdentityError> {
        let mut current = BTreeMap::new();
        let mut reviewed_subjects = BTreeSet::new();
        for case in cases {
            if !current_case(index, case)? {
                continue;
            }
            if current.insert(case.id.as_str(), case).is_some() {
                return Err(IdentityError::DuplicateReviewRecord(case.id.clone()));
            }
            if case.family == ATHLETE_IDENTITY_FAMILY {
                reviewed_subjects.insert(case.subject_id.as_str());
                reviewed_subjects.extend(case.member_ids.iter().map(|id| id.as_str()));
            }
        }
        let mut answers = BTreeMap::new();
        for verdict in verdicts {
            if answers.insert(verdict.case_id.as_str(), verdict).is_some() {
                return Err(IdentityError::DuplicateReviewRecord(
                    verdict.case_id.clone(),
                ));
            }
        }
        Ok(Self {
            cases: current,
            verdicts: answers,
            reviewed_subjects,
        })
    }
}
fn current_case(index: &AthleteIdentityIndex, case: &ReviewCase) -> Result<bool, IdentityError> {
    if case.state == ReviewState::Superseded {
        return Ok(false);
    }
    if case.family != ATHLETE_IDENTITY_FAMILY
        || case.member_ids.is_empty()
        || case
            .member_ids
            .iter()
            .any(|id| !index.facts.contains_key(id.as_str()))
    {
        return Ok(true);
    }
    let evidence = index.case_evidence(&case.subject, &case.detail, &case.member_ids)?;
    Ok(case.matches_evidence(&evidence))
}

pub(super) fn validate(
    index: &AthleteIdentityIndex,
    decision: &AppliedAthleteIdentity,
    reviews: &ReviewBindings<'_>,
) -> Result<Option<IdentityDecisionIssue>, IdentityError> {
    use IdentityDecisionIssue as Issue;
    if decision.policy != ATHLETE_IDENTITY_POLICY {
        return Ok(Some(Issue::WrongPolicy));
    }
    let ids: BTreeSet<_> = decision
        .members
        .iter()
        .map(|member| &member.subject)
        .collect();
    if ids.is_empty() || ids.len() != decision.members.len() {
        return Ok(Some(Issue::InvalidMembership));
    }
    for member in &decision.members {
        let Some(fact) = index.facts.get(member.subject.as_str()) else {
            return Ok(Some(Issue::InvalidMembership));
        };
        if fact.digest != member.evidence_digest {
            return Ok(Some(Issue::StaleMemberEvidence));
        }
        if fact.conflicted {
            return Ok(Some(Issue::RetainedConflict));
        }
    }
    if let Some(issue) = target_issue(decision, &ids) {
        return Ok(Some(issue));
    }
    if decision.kind == AppliedIdentityKind::SourceBound {
        let isolated = ids.first().is_some_and(|id| {
            index.isolated_source(id) && !reviews.reviewed_subjects.contains(id.as_str())
        });
        return Ok((!isolated).then_some(Issue::CompetingSourceClaims));
    }
    if let Some(issue) = review_issue(index, decision, reviews, &ids)? {
        return Ok(Some(issue));
    }
    Ok((!positive_identity(index, decision.kind, &ids))
        .then_some(Issue::MissingPositiveIdentityEvidence))
}

fn target_issue(
    decision: &AppliedAthleteIdentity,
    ids: &BTreeSet<&AthleteCandidateId>,
) -> Option<IdentityDecisionIssue> {
    use IdentityDecisionIssue as Issue;
    match decision.kind {
        AppliedIdentityKind::SourceBound => {
            if ids.len() != 1 || decision.case_id.is_some() || decision.verdict_digest.is_some() {
                return Some(Issue::InvalidMembership);
            }
        }
        AppliedIdentityKind::SamePerson | AppliedIdentityKind::DifferentPerson => {
            if ids.len() < 2 {
                return Some(Issue::InvalidMembership);
            }
        }
    }
    let expected = if decision.kind == AppliedIdentityKind::DifferentPerson {
        None
    } else {
        ids.first().map(|id| id.as_str())
    };
    (decision.canonical_id.as_ref().map(|id| id.as_str()) != expected)
        .then_some(Issue::InvalidCanonicalTarget)
}

fn review_issue(
    index: &AthleteIdentityIndex,
    decision: &AppliedAthleteIdentity,
    reviews: &ReviewBindings<'_>,
    ids: &BTreeSet<&AthleteCandidateId>,
) -> Result<Option<IdentityDecisionIssue>, IdentityError> {
    use IdentityDecisionIssue as Issue;
    let Some(case) = decision
        .case_id
        .as_deref()
        .and_then(|id| reviews.cases.get(id))
    else {
        return Ok(Some(Issue::MissingResolvedCase));
    };
    if case.state != ReviewState::Resolved
        || case.family != ATHLETE_IDENTITY_FAMILY
        || case.member_ids.iter().collect::<BTreeSet<_>>() != *ids
        || case.member_ids.len() != ids.len()
        || !case
            .member_ids
            .iter()
            .any(|id| id.as_str() == case.subject_id)
    {
        return Ok(Some(Issue::MissingResolvedCase));
    }
    let evidence = index.case_evidence(&case.subject, &case.detail, &case.member_ids)?;
    if !case.matches_evidence(&evidence) {
        return Ok(Some(Issue::StaleCaseEvidence));
    }
    let Some(verdict) = reviews.verdicts.get(case.id.as_str()) else {
        return Ok(Some(Issue::MissingAcceptedVerdict));
    };
    let expected = match decision.kind {
        AppliedIdentityKind::SamePerson => "same_person",
        AppliedIdentityKind::DifferentPerson => "different_person",
        AppliedIdentityKind::SourceBound => return Ok(Some(Issue::InvalidMembership)),
    };
    if !verdict.accepted
        || verdict.id != case.id
        || verdict.kind != "value_proposed"
        || verdict.field != "identity"
        || verdict.family != case.family
        || verdict.subject_id != case.subject_id
        || verdict.value != expected
        || verdict.member_ids.len() != ids.len()
        || verdict.member_ids.iter().collect::<BTreeSet<_>>() != *ids
        || decision.verdict_digest.as_deref() != Some(identity_verdict_digest(verdict)?.as_str())
    {
        return Ok(Some(Issue::MissingAcceptedVerdict));
    }
    Ok(None)
}

fn positive_identity(
    index: &AthleteIdentityIndex,
    kind: AppliedIdentityKind,
    ids: &BTreeSet<&AthleteCandidateId>,
) -> bool {
    let facts: Vec<_> = ids
        .iter()
        .filter_map(|id| index.facts.get(id.as_str()))
        .collect();
    if facts.len() != ids.len() {
        return false;
    }
    let Some(first) = facts.first() else {
        return false;
    };
    match kind {
        AppliedIdentityKind::SourceBound => false,
        AppliedIdentityKind::SamePerson => {
            let mut all_parsed = true;
            let mut all_same_grad_year = true;
            let mut has_boys = false;
            let mut has_girls = false;
            for fact in &facts {
                if !fact.parsed {
                    all_parsed = false;
                }
                if fact.grad_year != first.grad_year {
                    all_same_grad_year = false;
                }
                if fact.gender == super::Gender::Boys {
                    has_boys = true;
                }
                if fact.gender == super::Gender::Girls {
                    has_girls = true;
                }
            }
            all_parsed
                && all_same_grad_year
                && !(has_boys && has_girls)
                && facts.iter().all(|fact| first.shares_primary(fact))
        }
        AppliedIdentityKind::DifferentPerson => false,
    }
}

impl AthleteIdentityIndex {
    pub fn supports_identity(
        &self,
        kind: AppliedIdentityKind,
        members: &[AthleteCandidateId],
    ) -> bool {
        let ids: BTreeSet<_> = members.iter().collect();
        if ids.len() != members.len()
            || ids.iter().any(|id| {
                self.facts
                    .get(id.as_str())
                    .is_none_or(|fact| fact.conflicted)
            })
        {
            return false;
        }
        if kind == AppliedIdentityKind::SourceBound {
            ids.len() == 1 && ids.first().is_some_and(|id| self.isolated_source(id))
        } else {
            ids.len() > 1 && positive_identity(self, kind, &ids)
        }
    }
}

#[cfg(test)]
#[path = "identity_validation_tests.rs"]
mod tests;
