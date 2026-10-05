use census_domain::model::{
    CanonicalCoach, CoachContactClaim, CoachContactProgram, CoachRole, CoachTenure,
    CoachTenureEvidence, SchoolYear,
};

#[derive(Clone, Copy, Default)]
pub(super) struct Mailboxes<'a> {
    pub(super) professional: Option<&'a str>,
    pub(super) personal: Option<&'a str>,
}
pub(super) fn current_row(
    coach: &CanonicalCoach,
    school_year: SchoolYear,
) -> Option<Mailboxes<'_>> {
    let professional = normalized(coach.professional_email.as_deref());
    let personal = normalized(coach.personal_email.as_deref());
    let (current, mailboxes) = coach
        .tenure_evidence
        .iter()
        .filter_map(|fact| current_claim(fact, coach, school_year))
        .fold((false, Mailboxes::default()), |(_, found), claim| {
            let address = normalized(claim.mailbox.as_deref());
            (
                true,
                Mailboxes {
                    professional: found
                        .professional
                        .or(professional.filter(|value| Some(*value) == address)),
                    personal: found
                        .personal
                        .or(personal.filter(|value| Some(*value) == address)),
                },
            )
        });
    let name_only = professional.is_none() && personal.is_none();
    (current && (name_only || mailboxes.professional.is_some() || mailboxes.personal.is_some()))
        .then_some(mailboxes)
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
