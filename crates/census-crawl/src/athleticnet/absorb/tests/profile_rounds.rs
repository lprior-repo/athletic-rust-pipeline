use super::*;

fn published_rounds() -> Bio {
    serde_json::from_value(serde_json::json!({
        "athlete": {"IDAthlete": 28127170, "FirstName": "Natalia", "LastName": "Casillas", "Gender": "F", "SchoolID": 13850},
        "grades": {"13850_2026": 11},
        "allTeams": {"13850": {"SchoolName": "Seton Catholic"}},
        "allSeasons": [{"SchoolID": 13850, "IDSeason": 2026, "Display": "2026 Outdoor"}],
        "eventsTF": [{"IDEvent": 20, "Event": "200 Meters"}],
        "meets": {"589334": {"MeetName": "Invite", "EndDate": "2026-05-03T00:00:00"}},
        "resultsTF": [
            {"IDResult": 1, "Result": "26.10a", "FAT": 1, "Round": "F", "Division": "Varsity", "SchoolID": 13850, "EventID": 20, "MeetID": 589334, "SeasonID": 2026, "ResultDate": "2026-05-03"},
            {"IDResult": 2, "Result": "26.50a", "FAT": 1, "Round": "P", "Division": "Varsity", "SchoolID": 13850, "EventID": 20, "MeetID": 589334, "SeasonID": 2026, "ResultDate": "2026-05-03"}
        ]
    })).expect("published prelim and final marks")
}

#[test]
fn profile_rounds_keep_distinct_event_identities_and_result_references() {
    let (accumulated, outcome) = absorb_profile_for(&published_rounds(), 28127170);
    assert_eq!(
        outcome.expect("complete profile"),
        AbsorbOutcome::Complete { rows: 2 }
    );
    let events: std::collections::BTreeSet<_> = accumulated
        .performances
        .values()
        .map(|performance| {
            let event = accumulated
                .events
                .values()
                .find(|event| event.id == performance.event)
                .expect("result event retained");
            assert_eq!(event.round, performance.round);
            (
                event.round.as_deref().expect("published round"),
                event.id.as_str(),
            )
        })
        .collect();
    assert_eq!(
        events.iter().map(|(round, _)| *round).collect::<Vec<_>>(),
        ["final", "prelim"]
    );
    assert_ne!(
        events.first().expect("final").1,
        events.last().expect("prelim").1
    );
}

#[test]
fn an_unknown_profile_school_does_not_report_a_missing_target_state() {
    let mut bio = published_rounds();
    bio.teams.clear();
    let mut accumulated = Accumulator::default();
    let mut stats = Stats::default();
    let target = Target {
        athlete_id: 28127170,
        state: Some(UsJurisdiction::Wisconsin),
    };
    let source = SourceRef::new(
        "athleticnet",
        Some("https://www.athletic.net/athlete/28127170/tf".into()),
    );
    let outcome = absorb(
        &bio,
        Scope::TrackField,
        &target,
        AbsorbContext {
            source: &source,
            observed_on: "2026-09-30",
            index: &SchoolIndex::from_schools(&[]),
            resolved: &mut HashMap::new(),
            stats: &mut stats,
            accumulated: &mut accumulated,
        },
    )
    .expect("unresolved profile retained");
    assert!(matches!(outcome, AbsorbOutcome::Withheld { .. }));
    assert_eq!(stats.rows_unknown_school, 1);
    assert_eq!(stats.rows_without_state, 0);
    assert!(accumulated.performances.is_empty());
}

#[test]
fn missing_target_state_counts_each_withheld_result_not_the_profile() {
    for expected in [2_u64, 0] {
        let mut bio = published_rounds();
        if expected == 0 {
            bio.results_tf = None;
        }
        let mut accumulated = Accumulator::default();
        let mut stats = Stats::default();
        let target = Target {
            athlete_id: 28127170,
            state: None,
        };
        let source = SourceRef::new("athleticnet", Some(profile_url(target.athlete_id)));
        let outcome = absorb(
            &bio,
            Scope::TrackField,
            &target,
            AbsorbContext {
                source: &source,
                observed_on: "2026-10-01",
                index: &SchoolIndex::from_schools(&[]),
                resolved: &mut HashMap::new(),
                stats: &mut stats,
                accumulated: &mut accumulated,
            },
        )
        .expect("missing-state profile retained");
        assert!(matches!(outcome, AbsorbOutcome::Withheld { .. }));
        assert_eq!(stats.rows_without_state, expected);
        assert_eq!(stats.rows_seen, expected);
        assert_eq!(stats.rows_unknown_school, 0);
        assert!(accumulated.performances.is_empty());
    }
}
