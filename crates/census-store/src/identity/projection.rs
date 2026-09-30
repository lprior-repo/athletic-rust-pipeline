use crate::identity::aliases::IdentityAliases;
use crate::identity::validation::{validate, ReviewBindings};
use crate::identity::{AthleteIdentityIndex, IdentityError};
use census_domain::model::{
    AppliedAthleteIdentity, AppliedIdentityKind, AthleteCandidateId, IdentityDecisionIssue,
    IdentityStatus, ReviewCase, ReviewState, ReviewVerdictRecord, VerdictKind,
    ATHLETE_IDENTITY_FAMILY, CANONICAL_ID_COLLISION_FAMILY,
};
use std::collections::{BTreeMap, BTreeSet};

pub struct AthleteIdentityProjection {
    statuses: BTreeMap<AthleteCandidateId, IdentityStatus>,
    aliases: BTreeMap<AthleteCandidateId, AthleteCandidateId>,
    rejected_applications: BTreeMap<IdentityDecisionIssue, u64>,
}

impl AthleteIdentityProjection {
    pub fn status(&self, subject: &str) -> Result<IdentityStatus, IdentityError> {
        self.statuses
            .get(subject)
            .copied()
            .ok_or_else(|| IdentityError::UnknownSubject(subject.to_owned()))
    }

    pub fn canonical_id<'a>(&'a self, subject: &'a str) -> &'a str {
        self.aliases.get(subject).map_or(subject, |id| id.as_str())
    }

    pub fn rejected_applications(&self) -> &BTreeMap<IdentityDecisionIssue, u64> {
        &self.rejected_applications
    }
}

pub struct IdentityProjectionBuilder<'a> {
    pub(super) index: AthleteIdentityIndex,
    pub(super) reviews: ReviewBindings<'a>,
    aliases: IdentityAliases,
    different: Vec<Vec<AthleteCandidateId>>,
    rejected: BTreeMap<IdentityDecisionIssue, u64>,
}

impl<'a> IdentityProjectionBuilder<'a> {
    pub fn new(
        mut index: AthleteIdentityIndex,
        cases: &'a [ReviewCase],
        verdicts: &'a [ReviewVerdictRecord],
    ) -> Result<Self, IdentityError> {
        let reviews = ReviewBindings::new(&index, cases, verdicts)?;
        seed_statuses(&mut index, &reviews);
        Ok(Self {
            index,
            reviews,
            aliases: IdentityAliases::default(),
            different: Vec::new(),
            rejected: BTreeMap::new(),
        })
    }

    pub fn consider(
        &mut self,
        decision: &AppliedAthleteIdentity,
    ) -> Result<Option<IdentityDecisionIssue>, IdentityError> {
        let issue = self.decision_issue(decision)?;
        if let Some(issue) = issue {
            let count = self.rejected.entry(issue).or_default();
            *count = count.checked_add(1).ok_or(IdentityError::CounterOverflow)?;
            return Ok(Some(issue));
        }
        match decision.kind {
            AppliedIdentityKind::SourceBound => {}
            AppliedIdentityKind::SamePerson => {
                if let Some(first) = decision.members.first() {
                    for member in decision.members.iter().skip(1) {
                        self.aliases.join(&first.subject, &member.subject)?;
                    }
                }
            }
            AppliedIdentityKind::DifferentPerson => self.different.push(
                decision
                    .members
                    .iter()
                    .map(|member| member.subject.clone())
                    .collect(),
            ),
        }
        for member in &decision.members {
            let fact = self
                .index
                .facts
                .get_mut(member.subject.as_str())
                .ok_or_else(|| IdentityError::UnknownSubject(member.subject.to_string()))?;
            fact.authorized = true;
        }
        Ok(None)
    }

    pub(super) fn decision_issue(
        &self,
        decision: &AppliedAthleteIdentity,
    ) -> Result<Option<IdentityDecisionIssue>, IdentityError> {
        Ok(validate(&self.index, decision, &self.reviews)?.or_else(|| {
            decision
                .members
                .iter()
                .any(|member| {
                    self.index
                        .facts
                        .get(member.subject.as_str())
                        .is_some_and(|fact| fact.status > IdentityStatus::Verified)
                })
                .then_some(IdentityDecisionIssue::UnresolvedReview)
        }))
    }

    pub fn finish(self) -> Result<AthleteIdentityProjection, IdentityError> {
        let mut aliases = self.aliases.flattened()?;
        let conflicts = self.conflicting_roots(&aliases)?;
        aliases.retain(|_, root| !conflicts.contains(root));
        let mut rejected = self.rejected;
        if !conflicts.is_empty() {
            let count =
                u64::try_from(conflicts.len()).map_err(|_| IdentityError::CounterOverflow)?;
            rejected.insert(IdentityDecisionIssue::ConflictingApplications, count);
        }
        let facts = self.index.into_facts();
        let statuses = facts
            .into_iter()
            .map(|(id, fact)| {
                let root = self.aliases.root(&id)?;
                let status = if conflicts.contains(root) {
                    IdentityStatus::RetainedConflict
                } else if fact.status > IdentityStatus::Verified {
                    fact.status
                } else if fact.authorized {
                    IdentityStatus::Verified
                } else {
                    IdentityStatus::Unverified
                };
                Ok((id, status))
            })
            .collect::<Result<_, IdentityError>>()?;
        Ok(AthleteIdentityProjection {
            statuses,
            aliases,
            rejected_applications: rejected,
        })
    }

    fn conflicting_roots(
        &self,
        aliases: &BTreeMap<AthleteCandidateId, AthleteCandidateId>,
    ) -> Result<BTreeSet<AthleteCandidateId>, IdentityError> {
        let mut conflicts = BTreeSet::new();
        for distinct in &self.different {
            let mut seen = BTreeSet::new();
            for member in distinct {
                let root = self.aliases.root(member)?;
                if !seen.insert(root) {
                    conflicts.insert(root.clone());
                }
            }
        }
        let mut genders = BTreeMap::new();
        for (member, root) in aliases {
            let parent = self
                .index
                .facts
                .get(root.as_str())
                .ok_or_else(|| IdentityError::UnknownSubject(root.to_string()))?;
            let child = self
                .index
                .facts
                .get(member.as_str())
                .ok_or_else(|| IdentityError::UnknownSubject(member.to_string()))?;
            let gender = genders
                .entry(root)
                .or_insert_with(|| gender_bit(parent.gender));
            *gender |= gender_bit(child.gender);
            if *gender == 3 || child.grad_year != parent.grad_year {
                conflicts.insert(root.clone());
            }
        }
        Ok(conflicts)
    }
}
fn gender_bit(gender: census_domain::model::Gender) -> u8 {
    match gender {
        census_domain::model::Gender::Boys => 1,
        census_domain::model::Gender::Girls => 2,
        _ => 0,
    }
}

fn seed_statuses(index: &mut AthleteIdentityIndex, reviews: &ReviewBindings<'_>) {
    for fact in index.facts.values_mut().filter(|fact| fact.conflicted) {
        fact.status = IdentityStatus::RetainedConflict;
    }
    for case in reviews.cases.values().filter(|case| {
        matches!(
            case.family.as_str(),
            ATHLETE_IDENTITY_FAMILY | CANONICAL_ID_COLLISION_FAMILY
        )
    }) {
        let status = case_status(case, reviews.verdicts.get(case.id.as_str()).copied());
        let Some(status) = status else { continue };
        if case.member_ids.is_empty() {
            if let Some(fact) = index.facts.get_mut(case.subject_id.as_str()) {
                fact.status = fact.status.max(status);
            }
        } else {
            for member in &case.member_ids {
                if let Some(fact) = index.facts.get_mut(member.as_str()) {
                    fact.status = fact.status.max(status);
                }
            }
        }
    }
}

fn case_status(case: &ReviewCase, verdict: Option<&ReviewVerdictRecord>) -> Option<IdentityStatus> {
    match case.state {
        ReviewState::Superseded => None,
        ReviewState::Pending => Some(IdentityStatus::Pending),
        ReviewState::Retained => Some(if verdict.is_some_and(|row| !row.accepted) {
            IdentityStatus::Rejected
        } else {
            IdentityStatus::RetainedConflict
        }),
        ReviewState::Resolved => match verdict {
            Some(row)
                if row.accepted
                    && row.field == "identity"
                    && (row.value == VerdictKind::SamePerson.slug()
                        || row.value == VerdictKind::DifferentPerson.slug()) =>
            {
                None
            }
            Some(row) if !row.accepted => Some(IdentityStatus::Rejected),
            _ => Some(IdentityStatus::Pending),
        },
    }
}
