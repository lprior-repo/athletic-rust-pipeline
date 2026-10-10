use census_domain::model::{
    assess_coach_tenure, CanonicalCoach, CanonicalMeet, CoachContactClaim, CoachContactProgram,
    CoachRole, CoachTenure, CoachTenureEvidence, SchoolYear, SourceIdentity, SourceNamespace,
    TenureAssessmentError,
};

pub fn coach_source(coach: &CanonicalCoach) -> (Option<&str>, Option<&str>) {
    if let Some(fact) = newest_tenure_evidence(coach) {
        return (
            fact.source.url.as_deref(),
            Some(fact.retrieved_at.as_str()).filter(|date| !date.trim().is_empty()),
        );
    }
    if let Some(evidence) = newest_coach_evidence(coach) {
        return (
            evidence.source.url.as_deref(),
            Some(evidence.observed_on.as_str()).filter(|date| !date.is_empty()),
        );
    }
    (
        coach
            .source_identities
            .iter()
            .filter_map(|identity| identity.url.as_deref())
            .filter(|url| !url.trim().is_empty())
            .max(),
        None,
    )
}

fn newest_tenure_evidence(coach: &CanonicalCoach) -> Option<&CoachTenureEvidence> {
    coach
        .tenure_evidence
        .iter()
        .filter(|fact| {
            fact.source
                .url
                .as_deref()
                .is_some_and(|url| !url.trim().is_empty())
        })
        .max_by_key(|fact| {
            (
                fact.retrieved_at.as_str(),
                fact.source.url.as_deref(),
                fact.source_sha256.as_str(),
            )
        })
}

fn newest_coach_evidence(coach: &CanonicalCoach) -> Option<&census_domain::model::Evidence> {
    coach
        .evidence
        .iter()
        .filter(|evidence| {
            evidence
                .source
                .url
                .as_deref()
                .is_some_and(|url| !url.trim().is_empty())
        })
        .max_by_key(|evidence| {
            (
                evidence.observed_on.as_str(),
                evidence.source.url.as_deref(),
            )
        })
}

pub fn athletic_net_meet_identity(meet: &CanonicalMeet) -> Option<&SourceIdentity> {
    meet.source_identities.iter().find(|identity| {
        matches!(
            &identity.namespace,
            SourceNamespace::LegacyAthleticNet { kind } | SourceNamespace::AthleticNet { kind }
                if kind == "meet"
        )
    })
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContactSelectionError {
    #[error("current mailbox claims conflict")]
    MailboxConflict,
    #[error("the admitted mailbox capture has no source URL")]
    MissingCaptureUrl,
    #[error(transparent)]
    Tenure(#[from] TenureAssessmentError),
}

#[derive(Debug, Clone, Copy)]
pub struct SelectedContact<'a> {
    tenure: &'a CoachTenureEvidence,
    claim: &'a CoachContactClaim,
    mailbox: &'a str,
}

impl<'a> SelectedContact<'a> {
    pub const fn tenure(self) -> &'a CoachTenureEvidence {
        self.tenure
    }

    pub const fn claim(self) -> &'a CoachContactClaim {
        self.claim
    }

    pub const fn mailbox(self) -> &'a str {
        self.mailbox
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CurrentCoachContacts<'a> {
    professional: Option<SelectedContact<'a>>,
    personal: Option<SelectedContact<'a>>,
    name_only: Option<&'a CoachTenureEvidence>,
}

impl<'a> CurrentCoachContacts<'a> {
    pub const fn professional(self) -> Option<SelectedContact<'a>> {
        self.professional
    }

    pub const fn personal(self) -> Option<SelectedContact<'a>> {
        self.personal
    }

    pub const fn name_only(self) -> Option<&'a CoachTenureEvidence> {
        self.name_only
    }

    fn has_mailbox(self) -> bool {
        self.professional.is_some() || self.personal.is_some()
    }

    fn admit(&mut self, selected: SelectedContact<'a>, coach: &CanonicalCoach) -> bool {
        let professional = normalized(coach.professional_email.as_deref());
        let personal = normalized(coach.personal_email.as_deref());
        if professional == Some(selected.mailbox) {
            choose_contact(&mut self.professional, selected);
        }
        if personal == Some(selected.mailbox) {
            choose_contact(&mut self.personal, selected);
        }
        professional == Some(selected.mailbox) || personal == Some(selected.mailbox)
    }

    fn admit_claim(
        &mut self,
        fact: &'a CoachTenureEvidence,
        claim: &'a CoachContactClaim,
        coach: &CanonicalCoach,
    ) -> bool {
        match normalized(claim.mailbox.as_deref()) {
            Some(mailbox) => self.admit(
                SelectedContact {
                    tenure: fact,
                    claim,
                    mailbox,
                },
                coach,
            ),
            None => true,
        }
    }
}

enum MailboxAgreement {
    Consistent,
    Unmatched,
}

pub fn current_coach_contacts(
    coach: &CanonicalCoach,
    school_year: SchoolYear,
) -> Result<Option<CurrentCoachContacts<'_>>, ContactSelectionError> {
    if !assess_coach_tenure(&coach.tenure_evidence, school_year)?.is_current() {
        return Ok(None);
    }
    let mut contacts = CurrentCoachContacts::default();
    let mut agreement = MailboxAgreement::Consistent;
    for fact in &coach.tenure_evidence {
        let Some(claim) = current_claim(fact, coach, school_year) else {
            continue;
        };
        choose_fact(&mut contacts.name_only, fact);
        if !contacts.admit_claim(fact, claim, coach) {
            agreement = MailboxAgreement::Unmatched;
        }
    }
    finish_contacts(contacts, coach, agreement)
}

fn finish_contacts<'a>(
    contacts: CurrentCoachContacts<'a>,
    coach: &CanonicalCoach,
    agreement: MailboxAgreement,
) -> Result<Option<CurrentCoachContacts<'a>>, ContactSelectionError> {
    if contacts.has_mailbox() && matches!(agreement, MailboxAgreement::Unmatched) {
        return Err(ContactSelectionError::MailboxConflict);
    }
    for selected in [contacts.professional, contacts.personal]
        .into_iter()
        .flatten()
    {
        if normalized(selected.tenure.source.url.as_deref()).is_none() {
            return Err(ContactSelectionError::MissingCaptureUrl);
        }
    }
    let name_only = normalized(coach.professional_email.as_deref()).is_none()
        && normalized(coach.personal_email.as_deref()).is_none();
    Ok((contacts.has_mailbox() || (name_only && contacts.name_only.is_some())).then_some(contacts))
}

fn normalized(address: Option<&str>) -> Option<&str> {
    address.map(str::trim).filter(|value| !value.is_empty())
}

fn current_claim<'a>(
    fact: &'a CoachTenureEvidence,
    coach: &CanonicalCoach,
    school_year: SchoolYear,
) -> Option<&'a CoachContactClaim> {
    if !matches!(fact.tenure, CoachTenure::Current { school_year: year } if year == school_year) {
        return None;
    }
    let claim = fact.claim.as_ref()?;
    (claim.coach == coach.id
        && claim.school == coach.school
        && claim.role == coach.role
        && matching_program(claim, coach))
    .then_some(claim)
}

fn matching_program(claim: &CoachContactClaim, coach: &CanonicalCoach) -> bool {
    match (&claim.program, coach.role) {
        (CoachContactProgram::SchoolAthletics, CoachRole::AthleticDirector) => true,
        (
            CoachContactProgram::Team { sport, gender },
            CoachRole::HeadCoach | CoachRole::AssistantCoach,
        ) => coach.sport == Some(*sport) && coach.gender == *gender,
        _ => false,
    }
}

fn choose_contact<'a>(current: &mut Option<SelectedContact<'a>>, next: SelectedContact<'a>) {
    if current.is_none_or(|current| capture_key(next.tenure) > capture_key(current.tenure)) {
        *current = Some(next);
    }
}

fn choose_fact<'a>(current: &mut Option<&'a CoachTenureEvidence>, next: &'a CoachTenureEvidence) {
    if current.is_none_or(|current| capture_key(next) > capture_key(current)) {
        *current = Some(next);
    }
}

fn capture_key(fact: &CoachTenureEvidence) -> (&str, Option<&str>, &str, &str, &str) {
    (
        &fact.retrieved_at,
        fact.source.url.as_deref(),
        &fact.source_sha256,
        &fact.source.id,
        &fact.statement,
    )
}
