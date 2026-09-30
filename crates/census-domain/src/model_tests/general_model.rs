use super::*;

#[test]
fn graduation_follows_grade_and_school_year() -> Result<(), Box<dyn std::error::Error>> {
    let of = |grade, year| -> Result<_, Box<dyn std::error::Error>> {
        Ok(GradYear::of(
            Grade::new(grade).ok_or("invalid grade fixture")?,
            SchoolYear::new(year).ok_or("invalid school year fixture")?,
        ))
    };
    check!(eq; of(11, 2025)?, Some(GradYear::CO2027));
    check!(eq; of(12, 2025)?, GradYear::new(2026));
    check!(eq; of(12, 2026)?, Some(GradYear::CO2027));
    check!(eq; of(11, 2025)?.map(GradYear::get), Some(2027));
    let observed = ObservedGrade {
        grade: Grade::new(9).ok_or("invalid grade fixture")?,
        school_year: SchoolYear::new(2026).ok_or("invalid school year fixture")?,
        source: SourceRef::id("milesplit_roster"),
    };
    check!(eq; observed.grad_year(), GradYear::new(2030));
    check!(eq; observed.school_year.short(), "2026-27");
    Ok(())
}

#[test]
fn inferred_graduation_years_reject_unsupported_cohorts_without_clamping(
) -> Result<(), Box<dyn std::error::Error>> {
    for opening in SchoolYear::MIN_START_YEAR..=SchoolYear::MAX_START_YEAR {
        for raw_grade in 9..=12 {
            let grade = Grade::new(raw_grade).ok_or("invalid bounded grade fixture")?;
            let school_year =
                SchoolYear::new(opening).ok_or("invalid bounded school year fixture")?;
            let implied = opening + 13 - i16::from(raw_grade);
            let expected = GradYear::new(implied);
            let actual = GradYear::of(grade, school_year);
            check!(eq; actual, expected, "grade {raw_grade}, opening {opening}");
            if let Some(year) = actual {
                let encoded = serde_json::to_string(&year)?;
                check!(eq; serde_json::from_str::<GradYear>(&encoded)?, year);
            }
        }
    }
    Ok(())
}

#[test]
fn unsupported_grade_evidence_never_certifies_a_canonical_cohort(
) -> Result<(), Box<dyn std::error::Error>> {
    let (_, school) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Boundary High", "boundary", None);
    let mut athlete = CanonicalAthlete::new(
        &school,
        "Boundary Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "boundary-runner"),
    );
    check!(eq; athlete.derived_cohort_confidence(), None);
    athlete.observed_grades.push(ObservedGrade {
        grade: Grade::new(12).ok_or("invalid grade fixture")?,
        school_year: SchoolYear::new(SchoolYear::MAX_START_YEAR)
            .ok_or("invalid maximum school year fixture")?,
        source: SourceRef::id("milesplit_roster"),
    });
    check!(eq; athlete.derived_cohort_confidence(), Some(Confidence::LOW));
    athlete.observed_grades.push(ObservedGrade {
        grade: Grade::new(12).ok_or("invalid grade fixture")?,
        school_year: SchoolYear::new(2026).ok_or("invalid school year fixture")?,
        source: SourceRef::id("milesplit_roster"),
    });
    check!(eq; athlete.derived_cohort_confidence(), Some(Confidence::LOW));
    athlete.observed_grades.remove(0);
    check!(eq; athlete.derived_cohort_confidence(), Some(Confidence::HIGH));
    Ok(())
}

#[test]
fn school_year_flips_at_august_first() -> Result<(), Box<dyn std::error::Error>> {
    check!(eq; SchoolYear::containing(2025, 1), SchoolYear::new(2024));
    check!(eq; SchoolYear::containing(2025, 7), SchoolYear::new(2024));
    check!(eq; SchoolYear::containing(2025, 8), SchoolYear::new(2025));
    check!(eq; SchoolYear::containing(2025, 12), SchoolYear::new(2025));
    check!(eq; SchoolYear::containing(SchoolYear::MIN_START_YEAR, 1), None);
    Ok(())
}

#[test]
fn meet_identity_is_date_and_name_scoped() -> Result<(), Box<dyn std::error::Error>> {
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
    check!(eq; a, b);
    check!(ne; a, c);
    let venue = "La Crosse, WI";
    let d = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-29",
        "D3 Sectional #3",
        Some(venue),
    );
    check!(eq; a, d, "venue spelling must not fork meet identity");
    let level = CompetitionLevel::Sectional;
    let built = CanonicalMeet::new(
        Some(UsJurisdiction::Wisconsin),
        "D3 Sectional #3",
        "2026-05-29",
        level,
    );
    check!(eq; built.id, a, "constructor and mint must agree");
    check!(eq; built.state, Some(UsJurisdiction::Wisconsin));
    Ok(())
}

#[test]
fn an_unplaced_meet_remains_distinct_from_a_placed_meet() -> Result<(), Box<dyn std::error::Error>>
{
    let unplaced = CanonicalMeet::mint(None, "2026-05-29", "D3 Sectional #3", None);
    let placed = CanonicalMeet::mint(
        Some(UsJurisdiction::Wisconsin),
        "2026-05-29",
        "D3 Sectional #3",
    );
    check!(ne; unplaced, placed,
"placing the venue is what separates a coverage gap from a known jurisdiction");
    check!(eq; CanonicalMeet::new(
    None,
    "D3 Sectional #3",
    "2026-05-29",
    CompetitionLevel::Sectional
)
.state,
None);
    Ok(())
}

#[test]
fn a_legacy_unresolved_meet_state_decodes_to_none() -> Result<(), Box<dyn std::error::Error>> {
    use serde::de::value::{Error, StrDeserializer};
    let decode =
        |raw: &str| super::meet::deserialize_meet_state(StrDeserializer::<Error>::new(raw));
    check!(eq; decode(MEET_STATE_UNRESOLVED), Ok(None));
    check!(eq; decode("WI"), Ok(Some(UsJurisdiction::Wisconsin)));
    check!(eq; decode("wi"), Ok(Some(UsJurisdiction::Wisconsin)));
    check!(decode("PR").is_err(), "a territory is not a jurisdiction");
    Ok(())
}

#[test]
fn school_jurisdiction_separates_identity() -> Result<(), Box<dyn std::error::Error>> {
    let a = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
        None,
    );
    let b = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
        None,
    );
    check!(eq; a, b);
    let other = CanonicalSchool::mint(
        UsJurisdiction::Minnesota,
        "Abbotsford High School",
        "abbotsford",
        None,
    );
    check!(ne; a, other, "state participates in the natural key");
    let located = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
        Some("Abbotsford"),
    );
    check!(ne; a, located, "city participates in the natural key");
    let same_city = CanonicalSchool::mint(
        UsJurisdiction::Wisconsin,
        "Abbotsford High School",
        "abbotsford",
        Some("abbotsford"),
    );
    check!(eq; located, same_city);
    Ok(())
}

#[path = "general_model/published_identity.rs"]
mod published_identity;
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
