use athletic_rust_pipeline::domain::evidence::{
    BestClaim, EvidenceIssue, EvidenceRef, GradeAtSeason, Observed, ProfileEvidence,
    ResultAttribution, ResultEvidence, Sport, SportAvailability, TeamEvidence,
};
use athletic_rust_pipeline::domain::facts::{
    AthleteName, CityName, GraduationYear, Location, SchoolName,
};
use athletic_rust_pipeline::domain::identity::{AthleteId, EvidenceDigest, ProfileUrl};
use athletic_rust_pipeline::profile::merge_profiles;

fn digest(seed: u64) -> EvidenceDigest {
    EvidenceDigest::parse(&format!("{seed:064x}")).expect("digest")
}

fn evidence(seed: u64) -> EvidenceRef {
    EvidenceRef {
        document: digest(seed),
        locator: format!("body:{seed}"),
    }
}

fn observed<T>(value: T, seed: u64) -> Observed<T> {
    Observed {
        value,
        evidence: evidence(seed),
    }
}

fn profile() -> ProfileEvidence {
    ProfileEvidence {
        athlete_id: AthleteId::new(123).expect("athlete"),
        profile_url: ProfileUrl::parse("https://www.athletic.net/athlete/123/track-and-field/all")
            .expect("profile url"),
        name: observed(AthleteName::parse("Synthetic Runner").expect("name"), 1),
        teams: Vec::new(),
        graduation_years: Vec::new(),
        grades: Vec::new(),
        sports: Vec::new(),
        results: Vec::new(),
        issues: Vec::new(),
        documents: Vec::new(),
    }
}

fn team(team_id: u64, name: &str, seed: u64) -> TeamEvidence {
    TeamEvidence {
        team_id,
        name: observed(SchoolName::parse(name).expect("school"), seed),
        location: None,
        seasons: vec![2024],
        level: Some(4),
    }
}

fn team_in_city(team_id: u64, name: &str, city: &str, seed: u64) -> TeamEvidence {
    TeamEvidence {
        team_id,
        name: observed(SchoolName::parse(name).expect("school"), seed),
        location: Some(observed(
            Location::CityOnly(CityName::parse(city).expect("city")),
            seed,
        )),
        seasons: vec![2024],
        level: Some(4),
    }
}

fn team_without_level(team_id: u64, name: &str, seed: u64) -> TeamEvidence {
    TeamEvidence {
        team_id,
        name: observed(SchoolName::parse(name).expect("school"), seed),
        location: None,
        seasons: vec![2024],
        level: None,
    }
}

fn grade(team_id: u64, season: u16, value: u8, seed: u64) -> GradeAtSeason {
    GradeAtSeason {
        team_id,
        season,
        grade: value,
        evidence: evidence(seed),
    }
}

fn result(result_id: u64, mark: &str, seed: u64) -> ResultEvidence {
    ResultEvidence {
        result_id,
        sport: Sport::TrackField,
        event_id: Some(1),
        event_name: "100 Meters".to_owned(),
        event_description: None,
        event_type: Some("track".to_owned()),
        mark: mark.to_owned(),
        units: Some("seconds".to_owned()),
        season: 2024,
        team_id: 7,
        meet_id: 9,
        meet_name: Some("Synthetic Meet".to_owned()),
        date: Some("2024-05-01".to_owned()),
        wind: None,
        timing: Some("FAT".to_owned()),
        personal_best: BestClaim::Claimed,
        season_best: BestClaim::Claimed,
        attribution: ResultAttribution::Individual,
        short_code: None,
        result_url: None,
        evidence: evidence(seed),
    }
}

#[test]
fn distinct_ids_keep_left_first_order() {
    let mut left = profile();
    left.teams = vec![team(1, "One", 10)];
    left.results = vec![result(1, "10.5", 11)];
    let mut right = profile();
    right.teams = vec![team(2, "Two", 12), team(3, "Three", 13)];
    right.results = vec![result(2, "10.6", 14), result(3, "10.7", 15)];

    let merged = merge_profiles(left, right).expect("merge");

    assert_eq!(
        merged
            .teams
            .iter()
            .map(|value| value.team_id)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(
        merged
            .results
            .iter()
            .map(|value| value.result_id)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
}

#[test]
fn same_id_equivalence_ignores_provenance_but_conflicts_are_retained() {
    let mut left = profile();
    left.results = vec![result(7, "10.5", 20)];
    let mut right = profile();
    right.results = vec![
        result(7, "10.5", 21),
        result(7, "10.6", 22),
        result(7, "10.6", 23),
    ];

    let merged = merge_profiles(left, right).expect("merge");

    assert_eq!(merged.results.len(), 2);
    assert_eq!(merged.results[0].mark, "10.5");
    assert_eq!(merged.results[0].evidence.locator, "body:20");
    assert_eq!(merged.results[1].mark, "10.6");
    assert_eq!(
        merged
            .issues
            .iter()
            .filter(|issue| issue.code == "result_conflict")
            .count(),
        1
    );
}

#[test]
fn adversarial_same_id_buckets_still_issue_factual_conflicts() {
    let mut left = profile();
    left.teams = vec![team(9, "First", 30), team(9, "Second", 31)];
    left.grades = vec![grade(9, 2024, 10, 32), grade(9, 2024, 11, 33)];
    left.sports = vec![
        SportAvailability::ResultsObserved {
            sport: Sport::TrackField,
            count: 1,
        },
        SportAvailability::ResultsObserved {
            sport: Sport::TrackField,
            count: 2,
        },
    ];
    let mut right = profile();
    right.teams = vec![team(9, "First", 34)];
    right.grades = vec![grade(9, 2024, 10, 35)];
    right.sports = vec![SportAvailability::ResultsObserved {
        sport: Sport::TrackField,
        count: 1,
    }];

    let merged = merge_profiles(left, right).expect("merge");

    assert!(merged
        .issues
        .iter()
        .any(|issue| issue.code == "team_conflict"));
    assert!(merged
        .issues
        .iter()
        .any(|issue| issue.code == "grade_conflict"));
    assert!(merged
        .issues
        .iter()
        .any(|issue| issue.code == "sport_conflict"));
    assert_eq!(merged.teams.len(), 3);
    assert_eq!(merged.grades.len(), 3);
    assert_eq!(merged.sports.len(), 2);
}

#[test]
fn missing_team_location_or_level_does_not_conflict_with_present_observation() {
    let mut left = profile();
    left.teams = vec![team(14, "Same", 50)];
    let mut right = profile();
    right.teams = vec![team_in_city(14, "Same", "Austin", 51)];
    let merged = merge_profiles(left, right).expect("merge");
    assert!(!merged
        .issues
        .iter()
        .any(|issue| issue.code == "team_conflict"));

    let mut left = profile();
    left.teams = vec![team_in_city(15, "Same", "Austin", 52)];
    let mut right = profile();
    right.teams = vec![team(15, "Same", 53)];
    let merged = merge_profiles(left, right).expect("merge");
    assert!(!merged
        .issues
        .iter()
        .any(|issue| issue.code == "team_conflict"));

    let mut left = profile();
    left.teams = vec![team_without_level(16, "Same", 54)];
    let mut right = profile();
    right.teams = vec![team(16, "Same", 55)];
    let merged = merge_profiles(left, right).expect("merge");
    assert!(!merged
        .issues
        .iter()
        .any(|issue| issue.code == "team_conflict"));

    let mut left = profile();
    left.teams = vec![team(17, "Same", 56)];
    let mut right = profile();
    right.teams = vec![team_without_level(17, "Same", 57)];
    let merged = merge_profiles(left, right).expect("merge");
    assert!(!merged
        .issues
        .iter()
        .any(|issue| issue.code == "team_conflict"));
}

#[test]
fn duplicate_evidence_and_conflicts_are_retained_in_stable_order() {
    let mut left = profile();
    left.issues = vec![EvidenceIssue {
        code: "left_issue".to_owned(),
        message: "left".to_owned(),
        evidence: None,
    }];
    left.documents = vec![digest(40)];
    left.graduation_years = vec![observed(GraduationYear::new(2024).expect("year"), 41)];
    let mut right = profile();
    right.issues = vec![
        left.issues[0].clone(),
        EvidenceIssue {
            code: "right_issue".to_owned(),
            message: "right".to_owned(),
            evidence: None,
        },
    ];
    right.documents = vec![digest(40), digest(42)];
    right.graduation_years = vec![observed(GraduationYear::new(2024).expect("year"), 43)];

    let merged = merge_profiles(left, right).expect("merge");

    assert_eq!(
        merged
            .issues
            .iter()
            .map(|issue| issue.code.as_str())
            .collect::<Vec<_>>(),
        vec!["left_issue", "right_issue"]
    );
    assert_eq!(merged.documents, vec![digest(40), digest(42)]);
    assert_eq!(merged.graduation_years.len(), 2);
    assert!(!merged
        .issues
        .iter()
        .any(|issue| issue.code == "cohort_conflict"));
}
