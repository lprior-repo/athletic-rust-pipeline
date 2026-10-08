use super::*;
use census_domain::model::{EvidenceMethod, ReviewCase, SourceObservation};
use census_store::{Store, Table};

type RetainedGrade = (String, String, u8, i16, String);

fn retained_grades(accumulated: Accumulator) -> TestResult<Vec<RetainedGrade>> {
    let (_, observations) = accumulated.unsupported.into_parts();
    observations
        .into_iter()
        .map(|observation| {
            let SourceObservation::Athlete(row) = observation else {
                return Err("athlete observation".into());
            };
            let grade = row.observed_grade.ok_or("published grade")?;
            Ok((
                row.source_athlete_id,
                row.observed_school.ok_or("published grade school")?,
                grade.grade.get(),
                grade.school_year.get(),
                row.source_row_key,
            ))
        })
        .collect()
}

#[test]
fn absent_current_school_keeps_each_published_grade_school_and_locator() -> TestResult {
    let mut bio = retained_public_profile()?;
    bio.athlete.school_id = None;
    let (accumulated, outcome) = absorb_profile_for(&bio, 28872883);
    check!(eq;
        outcome?,
        AbsorbOutcome::Withheld {
            reason: "Missing or invalid published school",
        }
    );
    check!(accumulated.athletes.is_empty());
    check!(accumulated.performances.is_empty());
    check!(eq;
        retained_grades(accumulated)?,
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
    Ok(())
}

#[test]
fn malformed_grade_retains_its_value_and_locator_beside_valid_history() -> TestResult {
    let mut bio = retained_public_profile()?;
    bio.results_tf = None;
    bio.grades
        .as_mut()
        .ok_or("grades")?
        .insert("3204_2027".into(), 13);
    let (accumulated, outcome) = absorb_profile_for(&bio, 28872883);
    check!(eq;
        outcome?,
        AbsorbOutcome::Partial { rows: 0 }
    );
    let review = accumulated
        .profile_reviews
        .first()
        .ok_or("grade rejection")?;
    check!(eq;
        review.subject_id,
        "https://www.athletic.net/athlete/28872883/tf#grades/3204_2027"
    );
    check!(review
        .detail
        .contains("Invalid published grade; published grade 13"));
    check!(eq;
        retained_grades(accumulated)?
            .iter()
            .map(|row| (row.2, row.3))
            .collect::<Vec<_>>(),
        [(10, 2024), (11, 2025)]
    );
    Ok(())
}

#[test]
fn missing_grade_keeps_discovery_and_withholds_canonical_admission() -> TestResult {
    let mut bio = retained_public_profile()?;
    bio.grades = None;
    let (accumulated, outcome) = absorb_profile_for(&bio, 28872883);
    check!(eq;
        outcome?,
        AbsorbOutcome::Withheld {
            reason: "No valid published grade",
        }
    );
    check!(accumulated.athletes.is_empty());
    let discovery = accumulated
        .profile_observations
        .iter()
        .find_map(|observation| match observation {
            SourceObservation::Athlete(row) if row.source_row_key.ends_with("#grades") => Some(row),
            _ => None,
        })
        .ok_or("grade discovery")?;
    check!(eq; discovery.source_athlete_id, "28872883");
    check!(eq; discovery.observed_grade, None);
    check!(eq; discovery.observed_school.as_deref(), Some("Middleton"));
    check!(eq;
        discovery.profile_url.as_deref(),
        Some("https://www.athletic.net/athlete/28872883/track-and-field")
    );
    Ok(())
}

#[test]
fn profile_without_results_still_has_parsed_owner_support() -> TestResult {
    let mut bio = retained_public_profile()?;
    bio.results_tf = None;
    bio.results_xc = None;
    let (accumulated, outcome) = absorb_profile_for(&bio, 28872883);
    check!(eq;
        outcome?,
        AbsorbOutcome::Partial { rows: 0 }
    );
    let athlete = accumulated.athletes.values().next().ok_or("athlete")?;
    check!(eq; athlete.evidence.len(), 1);
    check!(eq;
        athlete.evidence.first().ok_or("support")?.method,
        EvidenceMethod::Parsed
    );
    let mut index = census_domain::model::AthleteIdentityIndex::default();
    index.observe(athlete)?;
    check!(index.isolated_source(&athlete.id.cast()));
    Ok(())
}

#[test]
fn withheld_grade_history_and_rejections_survive_the_same_batch_and_reopen() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut bio = retained_public_profile()?;
            bio.athlete.gender.clear();
            let (accumulated, outcome) = absorb_profile_for(&bio, 28872883);
            check!(eq;
                outcome?,
                AbsorbOutcome::Withheld {
                    reason: "Unknown published gender",
                }
            );
            let expected_reviews = accumulated.profile_reviews.clone();
            let dir = tempfile::tempdir()?;
            let root = dir.path().join("store");
            {
                let store = Store::open(&root)?;
                let fetcher = crate::net::Fetcher::new(
                    dir.path().join("http"),
                    None,
                    std::time::Duration::from_millis(1),
                    HashMap::new(),
                    Vec::new(),
                )?;
                let ctx = crate::AdapterContext {
                    fetcher: &fetcher,
                    store: &store,
                    refresh: false,
                    school_year: SchoolYear::new(2026).ok_or("school year")?,
                    observed_on: "2026-09-30".into(),
                    performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 9, 30)
                        .ok_or("snapshot date")?,
                    recording: None,
                };
                let mut batch = ctx.write_batch();
                crate::athleticnet::collect::store_accumulated(&ctx, accumulated, &mut batch)?;
                check!(eq;
                    store
                        .walk_table(Table::SourceObservations)
                        ?
                        .rows,
                    0
                );
                check!(eq;
                    store
                        .walk_table(Table::ReviewCases)
                        ?
                        .rows,
                    0
                );
                batch.commit()?;
            }
            let store = Store::open(&root)?;
            let snapshot = store.snapshot();
            let mut grades = Vec::new();
            snapshot.for_each_observation(Table::SourceObservations, |observation| {
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
            })?;
            grades.sort();
            check!(eq;
                grades,
                vec![
                    ("28872883".into(), Some("Middleton".into()), 10, 2024),
                    ("28872883".into(), Some("Middleton".into()), 11, 2025),
                ]
            );
            let reviews: Vec<ReviewCase> = snapshot.scan(Table::ReviewCases)?;
            for expected in expected_reviews {
                check!(
                    reviews.contains(&expected),
                    "missing {}",
                    expected.subject_id
                );
            }
            check!(snapshot.athletes()?.is_empty());
            Ok(())
        })
}

#[test]
fn unresolved_state_keeps_grade_facts_without_minting_a_school() -> TestResult {
    let bio = retained_public_profile()?;
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
            performance_as_of: super::fixture_cutoff()?,
            index: &SchoolIndex::from_schools(&[]),
            resolved: &mut HashMap::new(),
            stats: &mut Stats::default(),
            accumulated: &mut accumulated,
        },
    )?;
    check!(eq;
        outcome,
        AbsorbOutcome::Withheld {
            reason: "Unresolved profile school or state",
        }
    );
    check!(accumulated.schools.is_empty());
    check!(accumulated.athletes.is_empty());
    check!(accumulated.performances.is_empty());
    let grades = retained_grades(accumulated)?;
    check!(eq;
        grades
            .iter()
            .map(|row| (row.0.as_str(), row.1.as_str(), row.2, row.3))
            .collect::<Vec<_>>(),
        [
            ("28872883", "Middleton", 10, 2024),
            ("28872883", "Middleton", 11, 2025),
        ]
    );
    Ok(())
}
