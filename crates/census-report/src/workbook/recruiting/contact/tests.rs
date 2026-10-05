use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CoachContactClaim, CoachContactProgram, CoachRole,
    CoachTenure, CoachTenureEvidence, Evidence, Gender, GradYear, SchoolId, SchoolYear,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};

use super::{contacts, normalise::role_label, scoped, ContactState, Preferred, Slot};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod claim_binding;
mod conflicts;
mod persisted;
mod scopes;
mod tenure;

fn school() -> SchoolId {
    SchoolId::mint("sch", &["synthetic school"])
}

fn year() -> TestResult<SchoolYear> {
    SchoolYear::new(2026).ok_or_else(|| "invalid fixture season".into())
}

fn claim(tenure: CoachTenure) -> CoachTenureEvidence {
    CoachTenureEvidence {
        tenure,
        source: SourceRef::new(
            "synthetic_directory",
            Some("https://example.invalid/staff".into()),
        ),
        source_sha256: "a".repeat(64),
        retrieved_at: "2026-08-01T00:00:00Z".into(),
        statement: "Synthetic academic-year tenure statement".into(),
        claim: None,
    }
}

fn bind(coach: &mut CanonicalCoach) {
    let program = match (coach.role, coach.sport) {
        (CoachRole::AthleticDirector, _) => CoachContactProgram::SchoolAthletics,
        (CoachRole::HeadCoach | CoachRole::AssistantCoach, Some(sport)) => {
            CoachContactProgram::Team {
                sport,
                gender: coach.gender,
            }
        }
        _ => return,
    };
    let mailboxes: Vec<_> = coach
        .professional_email
        .iter()
        .chain(&coach.personal_email)
        .map(|mailbox| Some(mailbox.clone()))
        .collect();
    let mailboxes = if mailboxes.is_empty() {
        vec![None]
    } else {
        mailboxes
    };
    coach.tenure_evidence = coach
        .tenure_evidence
        .iter()
        .flat_map(|fact| {
            mailboxes.iter().map(|mailbox| {
                let mut fact = fact.clone();
                fact.claim = Some(CoachContactClaim {
                    coach: coach.id.clone(),
                    school: coach.school.clone(),
                    role: coach.role,
                    program: program.clone(),
                    mailbox: mailbox.clone(),
                });
                fact
            })
        })
        .collect();
}

fn person(
    name: &str,
    sport: Option<Sport>,
    side: Gender,
    role: CoachRole,
) -> TestResult<CanonicalCoach> {
    let mut coach = CanonicalCoach::new(&school(), name, sport, side, role);
    coach.tenure_evidence.push(claim(CoachTenure::Current {
        school_year: year()?,
    }));
    bind(&mut coach);
    Ok(coach)
}

fn head(name: &str, sport: Sport, side: Gender) -> TestResult<CanonicalCoach> {
    person(name, Some(sport), side, CoachRole::HeadCoach)
}

fn director() -> TestResult<CanonicalCoach> {
    person(
        "Current director",
        None,
        Gender::Mixed,
        CoachRole::AthleticDirector,
    )
}

fn athlete() -> CanonicalAthlete {
    let mut athlete = CanonicalAthlete::new(
        &school(),
        "Synthetic runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::Other("fixture".into()), "runner"),
    );
    athlete.sports = vec![Sport::OutdoorTrack];
    athlete
}

fn selected(rows: &[CanonicalCoach], athlete: &CanonicalAthlete) -> TestResult<Preferred> {
    let contacts = contacts(rows, year()?);
    Ok(scoped(contacts.get(athlete.school.as_str()), athlete).preferred())
}

#[test]
fn current_program_name_outranks_former_other_sport_email() -> TestResult {
    let current = head("Current coach", Sport::OutdoorTrack, Gender::Boys)?;
    let mut former = head("Former coach", Sport::CrossCountry, Gender::Boys)?;
    former.tenure_evidence = vec![claim(CoachTenure::Former {
        last_school_year: SchoolYear::new(2025),
    })];
    former.professional_email = Some("former@example.invalid".into());
    let result = selected(&[current, former], &athlete())?;
    check!(eq; result.name, "Current coach");
    check!(eq; result.email, "");
    check!(eq; result.state, ContactState::CoachNameOnly);
    Ok(())
}

#[test]
fn qualified_coach_mailboxes_outrank_director_without_becoming_professional() -> TestResult {
    for personal in [false, true] {
        let mut coach = head("Current coach", Sport::OutdoorTrack, Gender::Boys)?;
        let mut ad = director()?;
        ad.professional_email = Some("director@example.invalid".into());
        let expected = if personal {
            coach.personal_email = Some("coach@outlook.com".into());
            ContactState::PersonalCoachEmail
        } else {
            coach.professional_email = Some("coach@example.invalid".into());
            ContactState::ProfessionalCoachEmail
        };
        bind(&mut coach);
        bind(&mut ad);
        let contacts = contacts(&[coach, ad], year()?);
        let athlete = athlete();
        let scope = scoped(contacts.get(school().as_str()), &athlete);
        let result = scope.preferred();
        check!(eq; result.name, "Current coach");
        check!(eq; result.state, expected);
        check!(eq; scope.professional_coach_email(),
        (!personal).then_some("coach@example.invalid"));
    }
    Ok(())
}

#[test]
fn qualified_director_is_an_explicit_address_fallback_not_a_head_coach() -> TestResult {
    let coach = head("Named coach", Sport::OutdoorTrack, Gender::Boys)?;
    let mut ad = director()?;
    ad.professional_email = Some("director@example.invalid".into());
    bind(&mut ad);
    let result = selected(&[coach.clone(), ad.clone()], &athlete())?;
    check!(eq; result.name, "Current director");
    check!(eq; result.email, "director@example.invalid");
    check!(eq; result.state, ContactState::ProfessionalAdEmail);
    ad.professional_email = None;
    let result = selected(&[coach, ad.clone()], &athlete())?;
    check!(eq; result.name, "Named coach");
    check!(eq; result.state, ContactState::CoachNameOnly);
    let result = selected(&[ad], &athlete())?;
    check!(eq; result.name, "Current director");
    check!(eq; result.state, ContactState::AdNameOnly);
    Ok(())
}

#[test]
fn personal_director_address_never_enters_a_professional_coach_column() -> TestResult {
    let mut ad = director()?;
    ad.personal_email = Some("director@outlook.com".into());
    bind(&mut ad);
    let contacts = contacts(&[ad], year()?);
    let athlete = athlete();
    let scope = scoped(contacts.get(school().as_str()), &athlete);
    let result = scope.preferred();
    check!(eq; result.email, "director@outlook.com");
    check!(eq; result.state, ContactState::PersonalAdEmail);
    check!(eq; scope.professional_coach_email(), None);
    check!(eq; scope.director().ok_or("missing director")?.email, None);
    Ok(())
}

#[test]
fn missing_coach_rows_do_not_invent_a_research_attempt_or_a_negative_finding() -> TestResult {
    let athlete = athlete();
    check!(eq; scoped(None, &athlete).preferred().state,
    ContactState::ContactResearchUnknown);
    let assistant = person(
        "Assistant",
        Some(Sport::OutdoorTrack),
        Gender::Boys,
        CoachRole::AssistantCoach,
    )?;
    let result = selected(&[assistant], &athlete)?;
    check!(eq; result.state, ContactState::ContactResearchUnknown);
    check!(eq; result.name, "");
    check!(eq; result.email, "");
    Ok(())
}

#[test]
fn role_labels_name_the_slot_and_side_except_for_the_director() -> TestResult {
    for (slot, side, expected) in [
        (Slot::Director, Gender::Boys, "Athletic Director"),
        (Slot::Director, Gender::Girls, "Athletic Director"),
        (
            Slot::OutdoorTrack,
            Gender::Boys,
            "Head Outdoor TF Coach (boys)",
        ),
        (
            Slot::IndoorTrack,
            Gender::Girls,
            "Head Indoor TF Coach (girls)",
        ),
        (Slot::CrossCountry, Gender::Boys, "Head XC Coach (boys)"),
        (Slot::CrossCountry, Gender::Girls, "Head XC Coach (girls)"),
    ] {
        check!(eq; role_label(slot, side).as_str(), expected);
    }
    Ok(())
}

#[test]
fn merged_sources_take_the_newest_observation_and_larger_url_on_a_tied_date() -> TestResult {
    for (first_date, second_date, expected) in [
        ("2026-08-01", "2026-08-02", "https://example.invalid/second"),
        ("2026-08-02", "2026-08-02", "https://example.invalid/second"),
    ] {
        let mut first = head("Same coach", Sport::OutdoorTrack, Gender::Boys)?;
        first.evidence = vec![Evidence::parsed(
            SourceRef::new("fixture", Some("https://example.invalid/first".into())),
            first_date,
        )];
        let mut second = head("Same coach", Sport::OutdoorTrack, Gender::Boys)?;
        second.evidence = vec![Evidence::parsed(
            SourceRef::new("fixture", Some("https://example.invalid/second".into())),
            second_date,
        )];
        let result = selected(&[first, second], &athlete())?;
        check!(eq; result.source_url.as_str(), expected);
    }
    Ok(())
}
