use super::*;
use census_domain::model::{GradYear, Grade, SchoolYear, SourceIdentity, SourceNamespace};

const SAMPLE: &str =
    include_str!("../../../tests/fixtures/athleticlive_athletes/athlete-list-sample.json");

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

fn parsed_hits() -> Vec<AthleteHit> {
    let value: Value = serde_json::from_str(SAMPLE).expect("fixture is JSON");
    let sources: Vec<Value> = value["hits"]["hits"]
        .as_array()
        .expect("fixture has hits")
        .iter()
        .map(|hit| hit["_source"].clone())
        .collect();
    serde_json::from_value(Value::Array(sources)).expect("hits decode")
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
fn grade_is_interpreted_against_the_meet_school_year() {
    let fallback = SchoolYear(2026);
    // Grade 11 in a 2025-26 meet is class of 2027.
    let spring_2026 = school_year_for_date("2026-04-25", fallback);
    assert_eq!(GradYear::of(Grade::new(11).unwrap(), spring_2026).0, 2027);
    // Grade 12 in a 2026-27 meet is also class of 2027.
    let fall_2026 = school_year_for_date("2026-09-12", fallback);
    assert_eq!(GradYear::of(Grade::new(12).unwrap(), fall_2026).0, 2027);
    // A 2025 XC meet (2025-26) with grade 11 is class of 2027 too.
    let fall_2025 = school_year_for_date("2025-10-04", fallback);
    assert_eq!(GradYear::of(Grade::new(11).unwrap(), fall_2025).0, 2027);
    // Bad dates fall back rather than panicking.
    assert_eq!(school_year_for_date("", fallback).0, 2026);
    assert_eq!(school_year_for_date("garbage", fallback).0, 2026);
}

#[test]
fn rows_become_canonical_entities_with_athletic_net_seeds() {
    let hits = parsed_hits();
    assert!(!hits.is_empty(), "fixture must contain rows");
    let targets = targets_for();
    let by_id: HashMap<u64, &MeetTarget> = targets
        .iter()
        .map(|t| (t.athleticlive_meet_id, t))
        .collect();
    let entities = build_entities(&hits, &by_id, "2026-09-20", SchoolYear(2026));

    assert_eq!(entities.rows, hits.len());
    assert!(
        entities.rows_with_grade > 0,
        "the fixture carries graded rows for this meet"
    );
    assert!(!entities.athletes.is_empty(), "graded rows mint athletes");
    for athlete in &entities.athletes {
        // Fixture meet is 2025-04 (school year 2024-25): grade 9-12 maps to classes 2025-2028.
        assert!(
            (2024..=2031).contains(&athlete.grad_year.0),
            "grad year {} is outside the plausible window for this meet",
            athlete.grad_year.0
        );
        assert!(
            !athlete.public_profile_urls.is_empty(),
            "rows carrying an Athletic.net athlete id must expose the deterministic profile URL"
        );
        for url in &athlete.public_profile_urls {
            assert!(url.starts_with("https://www.athletic.net/athlete/"));
            assert!(url.ends_with("/track-and-field"));
        }
        assert!(
            athlete
                .observed_grades
                .iter()
                .all(|g| g.source.id == "athleticlive_athletes"),
            "grade observations keep their own source"
        );
    }
}

#[test]
fn one_athlete_seen_at_two_meets_stays_one_athlete() {
    let base = parsed_hits();
    let Some(first) = base.first().cloned() else {
        return;
    };
    let mut second = first.clone();
    second.mi = Some(json!(999_999));
    let first_meet_id = first.meet_id().unwrap();
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
    let entities = build_entities(&hits, &by_id, "2026-09-20", SchoolYear(2026));
    assert_eq!(entities.rows, 2);
    assert_eq!(
        entities.athletes.len(),
        1,
        "same school+name+grade+gender across meets is one canonical athlete"
    );
    assert_eq!(entities.teams.len(), 1, "and one canonical team");
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
    // The tenant meet index carries placeholder rows dated in the 2220s. A grade interpreted
    // against such a date would mint a class of 2223, so the meet is refused, not guessed at.
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
fn query_filters_grades_server_side() {
    let query = batch_query(&[73566, 75742], 2000);
    assert_eq!(query["from"], 2000);
    let filters = &query["query"]["bool"]["filter"];
    assert_eq!(filters[0]["terms"]["mi"][0], 73566);
    let grades = filters[1]["terms"]["y"].as_array().expect("grade terms");
    assert!(grades.iter().any(|v| v == "11"));
    assert!(grades.iter().any(|v| v == "JR"));
}

#[test]
fn empty_and_malformed_hits_yield_nothing_instead_of_panicking() {
    let targets = targets_for();
    let by_id: HashMap<u64, &MeetTarget> = targets
        .iter()
        .map(|t| (t.athleticlive_meet_id, t))
        .collect();
    let entities = build_entities(&[], &by_id, "2026-09-20", SchoolYear(2026));
    assert_eq!(entities.rows, 0);
    assert!(entities.athletes.is_empty());

    let nameless = AthleteHit {
        y: Some(Value::String("11".into())),
        ..Default::default()
    };
    let entities = build_entities(&[nameless], &by_id, "2026-09-20", SchoolYear(2026));
    assert_eq!(entities.rows, 1);
    assert!(
        entities.athletes.is_empty(),
        "a row without a name mints nothing"
    );

    let mut missing_school = parsed_hits().into_iter().next().unwrap();
    missing_school.t = None;
    let entities = build_entities(&[missing_school], &by_id, "2026-09-20", SchoolYear(2026));
    assert_eq!(entities.rows_without_school, 1);
    assert!(entities.athletes.is_empty());
}
