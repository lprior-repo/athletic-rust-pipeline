use super::identity_corroboration::{disjoint_provider_objects, member_facts, positive_identity};
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
    ConflictingProviderObjects,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerdictKind {
    SamePerson,
    DifferentPerson,
    Insufficient,
}

impl VerdictKind {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::SamePerson => "same_person",
            Self::DifferentPerson => "different_person",
            Self::Insufficient => "insufficient_evidence",
        }
    }
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
    let facts = member_facts(index, &ids);
    if disjoint_provider_objects(&facts) {
        return Ok(Some(Issue::ConflictingProviderObjects));
    }
    Ok((!positive_identity(decision.kind, &facts))
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
        AppliedIdentityKind::SamePerson => VerdictKind::SamePerson,
        AppliedIdentityKind::DifferentPerson => VerdictKind::DifferentPerson,
        AppliedIdentityKind::SourceBound => return Ok(Some(Issue::InvalidMembership)),
    };
    if !verdict.accepted
        || verdict.id != case.id
        || verdict.kind != "value_proposed"
        || verdict.field != "identity"
        || verdict.family != case.family
        || verdict.subject_id != case.subject_id
        || !verdict.value.eq(expected.slug())
        || verdict.member_ids.len() != ids.len()
        || verdict.member_ids.iter().collect::<BTreeSet<_>>() != *ids
        || decision.verdict_digest.as_deref() != Some(identity_verdict_digest(verdict)?.as_str())
    {
        return Ok(Some(Issue::MissingAcceptedVerdict));
    }
    Ok(None)
}

impl AthleteIdentityIndex {
    pub fn supports_identity(
        &self,
        kind: AppliedIdentityKind,
        members: &[AthleteCandidateId],
    ) -> bool {
        let ids: BTreeSet<_> = members.iter().collect();
        let facts = member_facts(self, &ids);
        if ids.len() != members.len()
            || facts.len() != ids.len()
            || facts.iter().any(|fact| fact.conflicted)
        {
            return false;
        }
        if kind == AppliedIdentityKind::SourceBound {
            ids.len() == 1 && ids.first().is_some_and(|id| self.isolated_source(id))
        } else {
            ids.len() > 1 && positive_identity(kind, &facts)
        }
    }
}

#[cfg(test)]
#[path = "identity_validation_tests.rs"]
mod tests;
