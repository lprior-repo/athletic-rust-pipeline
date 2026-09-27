use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CoachRole, CoachTenure, CoachTenureEvidence, Gender,
    GradYear, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, SourceRef, Sport,
};

use super::{contacts, scoped, ContactState, Preferred};

mod conflicts;
mod persisted;
mod scopes;
mod tenure;

fn school() -> SchoolId {
    SchoolId::mint("sch", &["synthetic school"])
}

fn year() -> SchoolYear {
    SchoolYear::new(2026).unwrap()
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
    }
}

fn person(name: &str, sport: Option<Sport>, side: Gender, role: CoachRole) -> CanonicalCoach {
    let mut coach = CanonicalCoach::new(&school(), name, sport, side, role);
    coach.tenure_evidence.push(claim(CoachTenure::Current {
        school_year: year(),
    }));
    coach
}

fn head(name: &str, sport: Sport, side: Gender) -> CanonicalCoach {
    person(name, Some(sport), side, CoachRole::HeadCoach)
}

fn director() -> CanonicalCoach {
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

fn selected(rows: &[CanonicalCoach], athlete: &CanonicalAthlete) -> Preferred {
    let contacts = contacts(rows, year());
    scoped(contacts.get(athlete.school.as_str()), athlete).preferred()
}

#[test]
fn current_program_name_outranks_former_other_sport_email() {
    let current = head("Current coach", Sport::OutdoorTrack, Gender::Boys);
    let mut former = head("Former coach", Sport::CrossCountry, Gender::Boys);
    former.tenure_evidence = vec![claim(CoachTenure::Former {
        last_school_year: SchoolYear::new(2025),
    })];
    former.professional_email = Some("former@example.invalid".into());
    let result = selected(&[current, former], &athlete());
    assert_eq!(result.name, "Current coach");
    assert_eq!(result.email, "");
    assert_eq!(result.state, ContactState::CoachNameOnly);
}

#[test]
fn qualified_coach_mailboxes_outrank_director_without_becoming_professional() {
    for personal in [false, true] {
        let mut coach = head("Current coach", Sport::OutdoorTrack, Gender::Boys);
        let mut ad = director();
        ad.professional_email = Some("director@example.invalid".into());
        let expected = if personal {
            coach.personal_email = Some("coach@outlook.com".into());
            ContactState::PersonalCoachEmail
        } else {
            coach.professional_email = Some("coach@example.invalid".into());
            ContactState::ProfessionalCoachEmail
        };
        let contacts = contacts(&[coach, ad], year());
        let athlete = athlete();
        let scope = scoped(contacts.get(school().as_str()), &athlete);
        let result = scope.preferred();
        assert_eq!(result.name, "Current coach");
        assert_eq!(result.state, expected);
        assert_eq!(
            scope.professional_coach_email(),
            (!personal).then_some("coach@example.invalid")
        );
    }
}

#[test]
fn qualified_director_is_an_explicit_address_fallback_not_a_head_coach() {
    let coach = head("Named coach", Sport::OutdoorTrack, Gender::Boys);
    let mut ad = director();
    ad.professional_email = Some("director@example.invalid".into());
    let result = selected(&[coach.clone(), ad.clone()], &athlete());
    assert_eq!(result.name, "Current director");
    assert_eq!(result.email, "director@example.invalid");
    assert_eq!(result.state, ContactState::ProfessionalAdEmail);
    ad.professional_email = None;
    let result = selected(&[coach, ad.clone()], &athlete());
    assert_eq!(result.name, "Named coach");
    assert_eq!(result.state, ContactState::CoachNameOnly);
    let result = selected(&[ad], &athlete());
    assert_eq!(result.name, "Current director");
    assert_eq!(result.state, ContactState::AdNameOnly);
}

#[test]
fn personal_director_address_never_enters_a_professional_coach_column() {
    let mut ad = director();
    ad.personal_email = Some("director@outlook.com".into());
    let contacts = contacts(&[ad], year());
    let athlete = athlete();
    let scope = scoped(contacts.get(school().as_str()), &athlete);
    let result = scope.preferred();
    assert_eq!(result.email, "director@outlook.com");
    assert_eq!(result.state, ContactState::PersonalAdEmail);
    assert_eq!(scope.professional_coach_email(), None);
    assert_eq!(scope.director().unwrap().email, None);
}

#[test]
fn missing_coach_rows_do_not_invent_a_research_attempt_or_a_negative_finding() {
    let athlete = athlete();
    assert_eq!(
        scoped(None, &athlete).preferred().state,
        ContactState::ContactResearchUnknown
    );
    let assistant = person(
        "Assistant",
        Some(Sport::OutdoorTrack),
        Gender::Boys,
        CoachRole::AssistantCoach,
    );
    let result = selected(&[assistant], &athlete);
    assert_eq!(result.state, ContactState::ContactResearchUnknown);
    assert_eq!(result.name, "");
    assert_eq!(result.email, "");
}
