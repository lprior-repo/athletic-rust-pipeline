use super::{
    identity_verdict_digest, AppliedAthleteIdentity, AppliedIdentityKind, AthleteCandidateId,
    IdentityDecisionIssue, IdentityError, IdentityProjectionBuilder, ReviewCase, ReviewState,
    VerdictKind, ATHLETE_IDENTITY_FAMILY, ATHLETE_IDENTITY_POLICY,
};

#[derive(Debug)]
pub struct AcceptedAthleteIdentity(AppliedAthleteIdentity);

impl AcceptedAthleteIdentity {
    pub fn record(&self) -> &AppliedAthleteIdentity {
        &self.0
    }
}

#[derive(Debug)]
pub enum IdentityApplication {
    Accepted(AcceptedAthleteIdentity),
    Retained(IdentityDecisionIssue),
}

impl IdentityProjectionBuilder<'_> {
    pub fn source_applications<'a>(
        &'a self,
        observed_at: &'a str,
    ) -> impl Iterator<Item = Result<IdentityApplication, IdentityError>> + 'a {
        self.index
            .subjects()
            .map(move |subject| self.source_application(subject, observed_at))
    }

    pub fn reviewed_applications<'a>(
        &'a self,
        observed_at: &'a str,
    ) -> impl Iterator<Item = (&'a ReviewCase, Result<IdentityApplication, IdentityError>)> + 'a
    {
        self.reviews
            .cases
            .values()
            .copied()
            .filter(|case| {
                case.family == ATHLETE_IDENTITY_FAMILY && case.state == ReviewState::Resolved
            })
            .map(move |case| (case, self.reviewed_application(case, observed_at)))
    }

    fn source_application(
        &self,
        subject: &AthleteCandidateId,
        observed_at: &str,
    ) -> Result<IdentityApplication, IdentityError> {
        if !self.index.isolated_source(subject) {
            return Ok(IdentityApplication::Retained(
                IdentityDecisionIssue::CompetingSourceClaims,
            ));
        }
        let member = self
            .index
            .member(subject)
            .ok_or_else(|| IdentityError::UnknownSubject(subject.to_string()))?;
        self.adjudicate(AppliedAthleteIdentity {
            id: format!(
                "source_bound:{subject}:p{ATHLETE_IDENTITY_POLICY}:{}",
                member.evidence_digest
            ),
            policy: ATHLETE_IDENTITY_POLICY,
            kind: AppliedIdentityKind::SourceBound,
            canonical_id: Some(subject.cast()),
            members: vec![member],
            case_id: None,
            verdict_digest: None,
            observed_at: observed_at.to_owned(),
        })
    }

    fn reviewed_application(
        &self,
        case: &ReviewCase,
        observed_at: &str,
    ) -> Result<IdentityApplication, IdentityError> {
        let Some(verdict) = self.reviews.verdicts.get(case.id.as_str()) else {
            return Ok(IdentityApplication::Retained(
                IdentityDecisionIssue::MissingAcceptedVerdict,
            ));
        };
        let same_slug = VerdictKind::SamePerson.slug();
        let different_slug = VerdictKind::DifferentPerson.slug();
        let kind = if verdict.value == same_slug {
            AppliedIdentityKind::SamePerson
        } else if verdict.value == different_slug {
            AppliedIdentityKind::DifferentPerson
        } else {
            return Ok(IdentityApplication::Retained(
                IdentityDecisionIssue::MissingAcceptedVerdict,
            ));
        };
        let Some(members) = case
            .member_ids
            .iter()
            .map(|subject| self.index.member(subject))
            .collect::<Option<Vec<_>>>()
        else {
            return Ok(IdentityApplication::Retained(
                IdentityDecisionIssue::InvalidMembership,
            ));
        };
        let canonical_id = (kind == AppliedIdentityKind::SamePerson)
            .then(|| {
                members
                    .iter()
                    .map(|member| &member.subject)
                    .min()
                    .map(|id| id.cast())
            })
            .flatten();
        let digest = identity_verdict_digest(verdict)?;
        self.adjudicate(AppliedAthleteIdentity {
            id: format!("identity:{}:p{ATHLETE_IDENTITY_POLICY}:{digest}", case.id),
            policy: ATHLETE_IDENTITY_POLICY,
            kind,
            members,
            canonical_id,
            case_id: Some(case.id.clone()),
            verdict_digest: Some(digest),
            observed_at: observed_at.to_owned(),
        })
    }

    fn adjudicate(
        &self,
        mut decision: AppliedAthleteIdentity,
    ) -> Result<IdentityApplication, IdentityError> {
        decision
            .members
            .sort_unstable_by(|left, right| left.subject.cmp(&right.subject));
        Ok(match self.decision_issue(&decision)? {
            Some(issue) => IdentityApplication::Retained(issue),
            None => IdentityApplication::Accepted(AcceptedAthleteIdentity(decision)),
        })
    }
}
