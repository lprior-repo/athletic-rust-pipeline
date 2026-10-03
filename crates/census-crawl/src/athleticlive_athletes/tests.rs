use super::*;
use census_domain::model::{GradYear, Grade, SchoolYear, SourceIdentity, SourceNamespace};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SAMPLE: &str =
    include_str!("../../tests/fixtures/athleticlive_athletes/athlete-list-sample.json");

fn targets_for() -> Vec<MeetTarget> {
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Kansas),
        "Abilene Invitational",
        "2025-04-25",
        census_domain::model::CompetitionLevel::Invitational,
    );
    meet.source_identities.push(SourceIdentity::new(
        SourceNamespace::TimerMeet {
            provider: "reddirt".into(),
        },
        "73566".to_string(),
    ));
    let meets = vec![meet];
    meet_targets(&meets, &[UsJurisdiction::Kansas]).targets
}

fn parsed_hits() -> TestResult<Vec<AthleteHit>> {
    let value: Value = serde_json::from_str(SAMPLE)?;
    let sources: Vec<Value> = value["hits"]["hits"]
        .as_array()
        .ok_or("fixture has hits")?
        .iter()
        .map(|hit| hit["_source"].clone())
        .collect();
    Ok(serde_json::from_value(Value::Array(sources))?)
}

#[test]
fn grade_tokens_cover_numeric_and_letter_encodings() {
    assert_eq!(grade_from_token("11").map(Grade::get), Some(11));
    assert_eq!(grade_from_token(" 12 ").map(Grade::get), Some(12));
    assert_eq!(grade_from_token("JR").map(Grade::get), Some(11));
    assert_eq!(grade_from_token("Sr").map(Grade::get), Some(12));
    assert_eq!(grade_from_token("FR").map(Grade::get), Some(9));
    assert_eq!(grade_from_token(""), None);
    assert_eq!(
        grade_from_token("5"),
        None,
        "middle-school grades are out of contract"
    );
    assert_eq!(grade_from_token("null"), None);
}

#[test]
fn grade_is_interpreted_against_the_meet_school_year() -> TestResult {
    let fallback = SchoolYear::new(2026).ok_or("2026 is a season")?;
    let spring_2026 = school_year_for_date("2026-04-25", fallback);
    check!(eq;
        GradYear::of(Grade::new(11).ok_or("junior grade")?, spring_2026)
            .ok_or("11th grade in 2026 has valid grad year")?
            .get(),
        2027
    );
    let fall_2026 = school_year_for_date("2026-09-12", fallback);
    check!(eq;
        GradYear::of(Grade::new(12).ok_or("senior grade")?, fall_2026)
            .ok_or("12th grade in 2026 has valid grad year")?
            .get(),
        2027
    );
    let fall_2025 = school_year_for_date("2025-10-04", fallback);
    check!(eq;
        GradYear::of(Grade::new(11).ok_or("junior grade")?, fall_2025)
            .ok_or("11th grade in 2025 has valid grad year")?
            .get(),
        2027
    );
    check!(eq; school_year_for_date("", fallback).get(), 2026);
    check!(eq; school_year_for_date("garbage", fallback).get(), 2026);
    check!(eq; school_year_for_date("1801-06-06", fallback).get(), 2026);
    Ok(())
}

#[test]
fn rows_become_canonical_entities_with_athletic_net_seeds() -> TestResult {
    let hits = parsed_hits()?;
    check!(!hits.is_empty(), "fixture must contain rows");
    let targets = targets_for();
    let by_id: HashMap<u64, &MeetTarget> = targets
        .iter()
        .map(|t| (t.athleticlive_meet_id, t))
        .collect();
    let entities = build_entities(
        &hits,
        &by_id,
        "2026-09-20",
        SchoolYear::new(2026).ok_or("2026 is a season")?,
    );

    check!(eq; entities.rows, hits.len());
    check!(
        entities.rows_with_grade > 0,
        "the fixture carries graded rows for this meet"
    );
    check!(!entities.athletes.is_empty(), "graded rows mint athletes");
    for athlete in &entities.athletes {
        check!(
            (2024..=2031).contains(&athlete.grad_year.get()),
            "grad year {} is outside the plausible window for this meet",
            athlete.grad_year.get()
        );
        check!(
            !athlete.public_profile_urls.is_empty(),
            "rows carrying an Athletic.net athlete id must expose the deterministic profile URL"
        );
        for url in &athlete.public_profile_urls {
            check!(url.starts_with("https://www.athletic.net/athlete/"));
            check!(url.ends_with("/track-and-field"));
        }
        check!(
            athlete
                .observed_grades
                .iter()
                .all(|g| g.source.id == "athleticlive_athletes"),
            "grade observations keep their own source"
        );
    }
    Ok(())
}

#[test]
fn one_athlete_seen_at_two_meets_stays_one_athlete() -> TestResult {
    let base = parsed_hits()?;
    let first = base.first().cloned().ok_or("fixture athlete")?;
    let mut second = first.clone();
    second.mi = Some(json!(999_999));
    let first_meet_id = first.meet_id().ok_or("fixture meet id")?;
    let hits = vec![first, second];
    let meet_a = MeetTarget {
        athleticlive_meet_id: first_meet_id,
        meet_id: "meet_a".into(),
        tenant: "reddirt".into(),
        name: "Abilene Invitational".into(),
        state: UsJurisdiction::Kansas,
        date: "2025-04-25".into(),
    };
    let meet_b = MeetTarget {
        athleticlive_meet_id: 999_999,
        meet_id: "meet_b".into(),
        tenant: "reddirt".into(),
        name: "Abilene Invitational".into(),
        state: UsJurisdiction::Kansas,
        date: "2025-04-25".into(),
    };
    let by_id: HashMap<u64, &MeetTarget> =
        [(meet_a.athleticlive_meet_id, &meet_a), (999_999, &meet_b)]
            .into_iter()
            .collect();
    let entities = build_entities(
        &hits,
        &by_id,
        "2026-09-20",
        SchoolYear::new(2026).ok_or("2026 is a season")?,
    );
    check!(eq; entities.rows, 2);
    check!(eq;
        entities.athletes.len(),
        1,
        "same school+name+grade+gender across meets is one canonical athlete"
    );
    check!(eq; entities.teams.len(), 1, "and one canonical team");
    Ok(())
}

#[test]
fn meet_targets_deduplicate_by_athleticlive_id_and_respect_state_filter() {
    let mut meet = CanonicalMeet::new(
        Some(UsJurisdiction::Kansas),
        "Abilene Invitational",
        "2025-04-25",
        census_domain::model::CompetitionLevel::Invitational,
    );
    meet.source_identities.push(SourceIdentity::new(
        SourceNamespace::TimerMeet {
            provider: "reddirt".into(),
        },
        "73566".to_string(),
    ));
    let mut duplicate = meet.clone();
    duplicate.source_identities = vec![SourceIdentity::new(
        SourceNamespace::TimerMeet {
            provider: "athleticlive".into(),
        },
        "73566".to_string(),
    )];
    let other_state = CanonicalMeet::new(
        Some(UsJurisdiction::SouthDakota),
        "Dakota XC",
        "2025-10-04",
        census_domain::model::CompetitionLevel::Invitational,
    );
    let mut other_state = other_state;
    other_state.source_identities.push(SourceIdentity::new(
        SourceNamespace::TimerMeet {
            provider: "dakota".into(),
        },
        "75742".to_string(),
    ));
    let targets = meet_targets(&[meet, duplicate, other_state], &[UsJurisdiction::Kansas]).targets;
    assert_eq!(
        targets.len(),
        1,
        "duplicate ids and other states are excluded"
    );
    assert_eq!(targets[0].athleticlive_meet_id, 73566);
}

#[test]
fn implausible_meet_dates_are_skipped_and_counted() {
    let mut placeholder = CanonicalMeet::new(
        Some(UsJurisdiction::Kansas),
        "Sample Meet",
        "2222-04-15",
        census_domain::model::CompetitionLevel::Invitational,
    );
    placeholder.source_identities.push(SourceIdentity::new(
        SourceNamespace::TimerMeet {
            provider: "athleticlive".into(),
        },
        "31939".to_string(),
    ));
    let mut real = CanonicalMeet::new(
        Some(UsJurisdiction::Kansas),
        "Abilene Invitational",
        "2025-04-25",
        census_domain::model::CompetitionLevel::Invitational,
    );
    real.source_identities.push(SourceIdentity::new(
        SourceNamespace::TimerMeet {
            provider: "reddirt".into(),
        },
        "73566".to_string(),
    ));
    let selection = meet_targets(&[placeholder, real], &[UsJurisdiction::Kansas]);
    assert_eq!(
        selection.targets.len(),
        1,
        "only the plausible meet survives"
    );
    assert_eq!(selection.targets[0].athleticlive_meet_id, 73566);
    assert_eq!(
        selection.skipped_implausible, 1,
        "the refusal is counted, not silent"
    );
}

#[test]
fn query_filters_grades_server_side() -> TestResult {
    let query = batch_query(&[73566, 75742], 2000);
    check!(eq; query["from"], 2000);
    let filters = &query["query"]["bool"]["filter"];
    check!(eq; filters[0]["terms"]["mi"][0], 73566);
    let grades = filters[1]["terms"]["y"].as_array().ok_or("grade terms")?;
    check!(grades.iter().any(|v| v == "11"));
    check!(grades.iter().any(|v| v == "JR"));
    Ok(())
}

#[test]
fn empty_and_malformed_hits_yield_nothing_instead_of_panicking() -> TestResult {
    let targets = targets_for();
    let by_id: HashMap<u64, &MeetTarget> = targets
        .iter()
        .map(|t| (t.athleticlive_meet_id, t))
        .collect();
    let entities = build_entities(
        &[],
        &by_id,
        "2026-09-20",
        SchoolYear::new(2026).ok_or("2026 is a season")?,
    );
    check!(eq; entities.rows, 0);
    check!(entities.athletes.is_empty());

    let nameless = AthleteHit {
        y: Some(Value::String("11".into())),
        ..Default::default()
    };
    let entities = build_entities(
        &[nameless],
        &by_id,
        "2026-09-20",
        SchoolYear::new(2026).ok_or("2026 is a season")?,
    );
    check!(eq; entities.rows, 1);
    check!(
        entities.athletes.is_empty(),
        "a row without a name mints nothing"
    );

    let mut missing_school = parsed_hits()?.into_iter().next().ok_or("fixture athlete")?;
    missing_school.t = None;
    let entities = build_entities(
        &[missing_school],
        &by_id,
        "2026-09-20",
        SchoolYear::new(2026).ok_or("2026 is a season")?,
    );
    check!(eq; entities.rows_without_school, 1);
    check!(entities.athletes.is_empty());
    Ok(())
}
