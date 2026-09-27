use super::*;

#[test]
fn graduation_follows_grade_and_school_year() {
    let of = |grade, year| GradYear::of(Grade::new(grade).unwrap(), SchoolYear::new(year).unwrap());
    assert_eq!(of(11, 2025), GradYear::CO2027);
    assert_eq!(of(12, 2025), GradYear::new(2026).unwrap());
    assert_eq!(of(12, 2026), GradYear::CO2027);
    assert_eq!(of(11, 2025).get(), 2027);
    let observed = ObservedGrade {
        grade: Grade::new(9).unwrap(),
        school_year: SchoolYear::new(2026).unwrap(),
        source: SourceRef::id("milesplit_roster"),
    };
    assert_eq!(observed.grad_year(), GradYear::new(2030).unwrap());
    assert_eq!(observed.school_year.short(), "2026-27");
}

#[test]
fn school_year_flips_at_august_first() {
    assert_eq!(
        SchoolYear::containing(2025, 1),
        Some(SchoolYear::new(2024).unwrap())
    );
    assert_eq!(
        SchoolYear::containing(2025, 7),
        Some(SchoolYear::new(2024).unwrap())
    );
    assert_eq!(
        SchoolYear::containing(2025, 8),
        Some(SchoolYear::new(2025).unwrap())
    );
    assert_eq!(
        SchoolYear::containing(2025, 12),
        Some(SchoolYear::new(2025).unwrap())
    );
    assert_eq!(SchoolYear::containing(SchoolYear::MIN_START_YEAR, 1), None);
}

#[test]
fn meet_identity_is_date_and_name_scoped() {
    let a = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-29",
        "D3 Sectional #3",
        None,
    );
    let b = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-29",
        "D3 sectional #3",
        None,
    );
    let c = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-30",
        "D3 Sectional #3",
        None,
    );
    assert_eq!(a, b);
    assert_ne!(a, c);
    let venue = "La Crosse, WI";
    let d = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-29",
        "D3 Sectional #3",
        Some(venue),
    );
    assert_eq!(a, d, "venue spelling must not fork meet identity");
    let level = CompetitionLevel::Sectional;
    let built = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "D3 Sectional #3",
        "2026-05-29",
        level,
    );
    assert_eq!(built.id, a, "constructor and mint must agree");
    assert_eq!(built.state, Some(UsJurisdiction::Wisconsin));
}

#[test]
fn an_unplaced_meet_remains_distinct_from_a_placed_meet() {
    let unplaced = CanonicalMeet::mint(None, "2026-05-29", "D3 Sectional #3", None);
    let placed = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-29",
        "D3 Sectional #3",
        None,
    );
    assert_ne!(
        unplaced, placed,
        "placing the venue is what separates a coverage gap from a known jurisdiction"
    );
    assert_eq!(
        CanonicalMeet::new(
            None,
            "D3 Sectional #3",
            "2026-05-29",
            CompetitionLevel::Sectional
        )
        .state,
        None
    );
}

#[test]
fn a_legacy_unresolved_meet_state_decodes_to_none() {
    use serde::de::value::{Error, StrDeserializer};
    let decode =
        |raw: &str| super::meet::deserialize_meet_state(StrDeserializer::<Error>::new(raw));
    assert_eq!(decode(MEET_STATE_UNRESOLVED), Ok(None));
    assert_eq!(decode("WI"), Ok(Some(UsJurisdiction::Wisconsin)));
    assert_eq!(decode("wi"), Ok(Some(UsJurisdiction::Wisconsin)));
    assert!(decode("PR").is_err(), "a territory is not a jurisdiction");
}

#[test]
fn school_jurisdiction_separates_identity() {
    let a = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
    );
    let b = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
    );
    assert_eq!(a, b);
    let other = CanonicalSchool::mint(
        UsJurisdiction::Minnesota,
        "Abbotsford High School",
        "abbotsford",
    );
    assert_ne!(a, other, "state participates in the natural key");
}
