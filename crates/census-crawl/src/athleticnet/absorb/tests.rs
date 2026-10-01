use super::*;
use census_domain::UsJurisdiction;

#[test]
fn conflicting_profile_grades_preserve_both_observations_and_sources() {
    let mut accumulated = Accumulator::default();
    let mut resolved = HashMap::new();
    let mut stats = Stats::default();
    let index = SchoolIndex::from_schools(&[]);
    let target = Target {
        athlete_id: 28127170,
        state: Some(UsJurisdiction::Alaska),
    };
    for (grade, scope) in [(11, Scope::TrackField), (10, Scope::CrossCountry)] {
        let bio: Bio = serde_json::from_value(serde_json::json!({
            "athlete": {"IDAthlete": 28127170, "FirstName": "Natalia",
                "LastName": "Casillas", "Gender": "F", "SchoolID": 13850},
            "grades": {"13850_2026": grade},
            "allTeams": {"13850": {"IDSchool": 13850, "SchoolName": "Seton Catholic"}},
            "allSeasons": []
        }))
        .expect("profile");
        let source = SourceRef::new(
            "athleticnet",
            Some(format!(
                "https://www.athletic.net/athlete/28127170/{}",
                scope.parameter()
            )),
        );
        absorb(
            &bio,
            scope,
            &target,
            &source,
            "2026-09-30",
            &index,
            &mut resolved,
            &mut stats,
            &mut accumulated,
        );
    }
    let athlete = accumulated.athletes.values().next().expect("athlete");
    let years: Vec<_> = athlete
        .observed_grades
        .iter()
        .map(|grade| grade.grad_year().map(GradYear::get))
        .collect();
    assert_eq!(years, vec![Some(2027), Some(2028)]);
    let urls: Vec<_> = athlete
        .evidence
        .iter()
        .map(|evidence| evidence.source.url.as_deref())
        .collect();
    assert_eq!(
        urls,
        vec![
            Some("https://www.athletic.net/athlete/28127170/tf"),
            Some("https://www.athletic.net/athlete/28127170/xc"),
        ]
    );
}

#[test]
fn unsupported_latest_grade_does_not_discard_earlier_supported_evidence() {
    let bio: Bio = serde_json::from_value(serde_json::json!({
        "athlete": {"IDAthlete": 28127170, "FirstName": "Natalia",
            "LastName": "Casillas", "Gender": "F", "SchoolID": 13850},
        "grades": {"13850_2026": 11, "13850_2041": 12},
        "allTeams": {"13850": {"IDSchool": 13850, "SchoolName": "Seton Catholic"}},
        "allSeasons": []
    }))
    .expect("profile");
    let mut accumulated = Accumulator::default();
    let target = Target {
        athlete_id: 28127170,
        state: Some(UsJurisdiction::Alaska),
    };
    let source = SourceRef::new("athleticnet", Some(profile_url(target.athlete_id)));
    absorb(
        &bio,
        Scope::TrackField,
        &target,
        &source,
        "2026-09-30",
        &SchoolIndex::from_schools(&[]),
        &mut HashMap::new(),
        &mut Stats::default(),
        &mut accumulated,
    );
    assert!(accumulated.athletes.is_empty());
    let (cases, rows) = accumulated.unsupported.into_parts();
    assert_eq!(cases.len(), 1);
    let years: Vec<_> = rows
        .into_iter()
        .map(|row| {
            let census_domain::model::SourceObservation::Athlete(row) = row else {
                panic!("athlete evidence required");
            };
            row.observed_grade
                .expect("original grade")
                .grad_year()
                .map(GradYear::get)
        })
        .collect();
    assert_eq!(years, vec![Some(2027), None]);
}
