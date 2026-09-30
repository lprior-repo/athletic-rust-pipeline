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
    );
    let b = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-29",
        "D3 sectional #3",
    );
    let c = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-30",
        "D3 Sectional #3",
    );
    assert_eq!(a, b);
    assert_ne!(a, c);
    let level = CompetitionLevel::Sectional;
    let built = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "D3 Sectional #3",
        "2026-05-29",
        level,
    );
    assert_eq!(built.id, a, "constructor and mint must agree");
    assert_eq!(built.state, Some(UsJurisdiction::Wisconsin));
    let mut d = built.clone();
    d.location = Some("La Crosse, WI".to_string());
    assert_eq!(built.id, d.id, "venue spelling must not fork meet identity");
}

#[test]
fn an_unplaced_meet_remains_distinct_from_a_placed_meet() {
    let unplaced = CanonicalMeet::mint(None, "2026-05-29", "D3 Sectional #3");
    let placed = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-29",
        "D3 Sectional #3",
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

#[test]
fn a_meet_keeps_a_year_only_source_date() {
    let meet = CanonicalMeet::new_checked(
        Some(UsJurisdiction::Wisconsin),
        "Racine Sectional",
        "2023",
        CompetitionLevel::Sectional,
    )
    .expect("a source date published only as a year is year precision");
    assert_eq!(meet.date, "2023");
}

#[test]
fn a_meet_refuses_a_date_that_is_neither_an_iso_day_nor_a_year() {
    for date in ["2023-13", "2023-02-30", "23", "spring 2023", ""] {
        assert!(
            CanonicalMeet::new_checked(
                Some(UsJurisdiction::Wisconsin),
                "Racine Sectional",
                date,
                CompetitionLevel::Sectional,
            )
            .is_err(),
            "{date:?} is neither an ISO day nor a four-digit year"
        );
    }
}

#[test]
fn a_performance_keeps_a_year_only_source_date() {
    let athlete = AthleteId::mint("ath", &["year-only"]);
    let team = TeamId::mint("team", &["year-only"]);
    let meet = MeetId::mint("meet", &["year-only"]);
    let event = EventId::mint("event", &["year-only"]);
    let performance = CanonicalPerformance::new_checked(
        &athlete,
        &EventKind::CrossCountry,
        &team,
        &event,
        &meet,
        "2023",
        Mark::Raw("16:00".to_string()),
        "year-only-key",
        None,
        None,
    )
    .expect("a source date published only as a year is year precision");
    assert_eq!(performance.date, "2023");
}
