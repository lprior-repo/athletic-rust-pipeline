use super::*;
use census_domain::model::{EvidenceMethod, ReviewCase, SourceObservation};
use census_store::{Store, Table};

fn retained_grades(accumulated: Accumulator) -> Vec<(String, String, u8, i16, String)> {
    let (_, observations) = accumulated.unsupported.into_parts();
    observations
        .into_iter()
        .map(|observation| {
            let SourceObservation::Athlete(row) = observation else {
                panic!("athlete observation");
            };
            let grade = row.observed_grade.expect("published grade");
            (
                row.source_athlete_id,
                row.observed_school.expect("published grade school"),
                grade.grade.get(),
                grade.school_year.get(),
                row.source_row_key,
            )
        })
        .collect()
}

#[test]
fn absent_current_school_keeps_each_published_grade_school_and_locator() {
    let mut bio = retained_public_profile();
    bio.athlete.school_id = None;
    let (accumulated, outcome) = absorb_profile_for(&bio, 28872883);
    assert_eq!(
        outcome.expect("retained"),
        AbsorbOutcome::Withheld {
            reason: "Missing or invalid published school",
        }
    );
    assert!(accumulated.athletes.is_empty());
    assert!(accumulated.performances.is_empty());
    assert_eq!(
        retained_grades(accumulated),
        vec![
            (
                "28872883".into(),
                "Middleton".into(),
                10,
                2024,
                "https://www.athletic.net/athlete/28872883/tf#grades/3204_2025".into()
            ),
            (
                "28872883".into(),
                "Middleton".into(),
                11,
                2025,
                "https://www.athletic.net/athlete/28872883/tf#grades/3204_2026".into()
            ),
        ]
    );
}

#[test]
fn malformed_grade_retains_its_value_and_locator_beside_valid_history() {
    let mut bio = retained_public_profile();
    bio.results_tf = None;
    bio.grades
        .as_mut()
        .expect("grades")
        .insert("3204_2027".into(), 13);
    let (accumulated, outcome) = absorb_profile_for(&bio, 28872883);
    assert_eq!(
        outcome.expect("partial"),
        AbsorbOutcome::Partial { rows: 0 }
    );
    let review = accumulated
        .profile_reviews
        .first()
        .expect("grade rejection");
    assert_eq!(
        review.subject_id,
        "https://www.athletic.net/athlete/28872883/tf#grades/3204_2027"
    );
    assert!(review
        .detail
        .contains("Invalid published grade; published grade 13"));
    assert_eq!(
        retained_grades(accumulated)
            .iter()
            .map(|row| (row.2, row.3))
            .collect::<Vec<_>>(),
        [(10, 2024), (11, 2025)]
    );
}

#[test]
fn missing_grade_keeps_discovery_and_withholds_canonical_admission() {
    let mut bio = retained_public_profile();
    bio.grades = None;
    let (accumulated, outcome) = absorb_profile_for(&bio, 28872883);
    assert_eq!(
        outcome.expect("retained"),
        AbsorbOutcome::Withheld {
            reason: "No valid published grade",
        }
    );
    assert!(accumulated.athletes.is_empty());
    let discovery = accumulated
        .profile_observations
        .iter()
        .find_map(|observation| match observation {
            SourceObservation::Athlete(row) if row.source_row_key.ends_with("#grades") => Some(row),
            _ => None,
        })
        .expect("grade discovery");
    assert_eq!(discovery.source_athlete_id, "28872883");
    assert_eq!(discovery.observed_grade, None);
    assert_eq!(discovery.observed_school.as_deref(), Some("Middleton"));
    assert_eq!(
        discovery.profile_url.as_deref(),
        Some("https://www.athletic.net/athlete/28872883/track-and-field")
    );
}

#[test]
fn profile_without_results_still_has_parsed_owner_support() {
    let mut bio = retained_public_profile();
    bio.results_tf = None;
    bio.results_xc = None;
    let (accumulated, outcome) = absorb_profile_for(&bio, 28872883);
    assert_eq!(
        outcome.expect("admitted"),
        AbsorbOutcome::Complete { rows: 0 }
    );
    let athlete = accumulated.athletes.values().next().expect("athlete");
    assert_eq!(athlete.evidence.len(), 1);
    assert_eq!(
        athlete.evidence.first().expect("support").method,
        EvidenceMethod::Parsed
    );
    let mut index = census_domain::model::AthleteIdentityIndex::default();
    index.observe(athlete).expect("identity evidence");
    assert!(index.isolated_source(&athlete.id.cast()));
}

#[tokio::test]
async fn withheld_grade_history_and_rejections_survive_the_same_batch_and_reopen() {
    let mut bio = retained_public_profile();
    bio.athlete.gender.clear();
    let (accumulated, outcome) = absorb_profile_for(&bio, 28872883);
    assert_eq!(
        outcome.expect("retained"),
        AbsorbOutcome::Withheld {
            reason: "Unknown published gender",
        }
    );
    let expected_reviews = accumulated.profile_reviews.clone();
    let dir = tempfile::tempdir().expect("isolated store");
    let root = dir.path().join("store");
    {
        let store = Store::open(&root).expect("store");
        let fetcher = crate::net::Fetcher::new(
            dir.path().join("http"),
            None,
            std::time::Duration::from_millis(1),
            HashMap::new(),
            Vec::new(),
        )
        .expect("fetcher");
        let ctx = crate::AdapterContext {
            fetcher: &fetcher,
            store: &store,
            refresh: false,
            school_year: SchoolYear::new(2026).expect("school year"),
            observed_on: "2026-09-30".into(),
            recording: None,
        };
        let mut batch = ctx.write_batch();
        crate::athleticnet::collect::store_accumulated(&ctx, accumulated, &mut batch)
            .expect("staged evidence");
        assert_eq!(
            store
                .walk_table(Table::SourceObservations)
                .expect("uncommitted")
                .rows,
            0
        );
        assert_eq!(
            store
                .walk_table(Table::ReviewCases)
                .expect("uncommitted")
                .rows,
            0
        );
        batch.commit().expect("committed together");
    }
    let store = Store::open(&root).expect("reopened");
    let snapshot = store.snapshot();
    let mut grades = Vec::new();
    snapshot
        .for_each_observation(Table::SourceObservations, |observation| {
            if let SourceObservation::Athlete(row) = observation {
                if let Some(grade) = row.observed_grade {
                    grades.push((
                        row.source_athlete_id,
                        row.observed_school,
                        grade.grade.get(),
                        grade.school_year.get(),
                    ));
                }
            }
            Ok(())
        })
        .expect("raw retained history");
    grades.sort();
    assert_eq!(
        grades,
        vec![
            ("28872883".into(), Some("Middleton".into()), 10, 2024),
            ("28872883".into(), Some("Middleton".into()), 11, 2025),
        ]
    );
    let reviews: Vec<ReviewCase> = snapshot
        .scan(Table::ReviewCases)
        .expect("retained rejections");
    for expected in expected_reviews {
        assert!(
            reviews.contains(&expected),
            "missing {}",
            expected.subject_id
        );
    }
    assert!(snapshot.athletes().expect("athletes").is_empty());
}

#[test]
fn unresolved_state_keeps_grade_facts_without_minting_a_school() {
    let bio = retained_public_profile();
    let target = Target {
        athlete_id: 28872883,
        state: None,
    };
    let source = SourceRef::new("athleticnet", Some(profile_url(28872883)));
    let mut accumulated = Accumulator::default();
    let outcome = absorb(
        &bio,
        Scope::TrackField,
        &target,
        AbsorbContext {
            source: &source,
            observed_on: "2026-09-30",
            index: &SchoolIndex::from_schools(&[]),
            resolved: &mut HashMap::new(),
            stats: &mut Stats::default(),
            accumulated: &mut accumulated,
        },
    )
    .expect("retained grades");
    assert_eq!(
        outcome,
        AbsorbOutcome::Withheld {
            reason: "Unresolved profile school or state",
        }
    );
    assert!(accumulated.schools.is_empty());
    assert!(accumulated.athletes.is_empty());
    assert!(accumulated.performances.is_empty());
    let grades = retained_grades(accumulated);
    assert_eq!(
        grades
            .iter()
            .map(|row| (row.0.as_str(), row.1.as_str(), row.2, row.3))
            .collect::<Vec<_>>(),
        [
            ("28872883", "Middleton", 10, 2024),
            ("28872883", "Middleton", 11, 2025),
        ]
    );
}
