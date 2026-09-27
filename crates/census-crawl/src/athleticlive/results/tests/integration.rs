use super::*;

#[tokio::test]
async fn collect_maps_a_captured_state_final_into_the_canonical_tables() {
    let (dir, store, fetcher) = scratch();
    let doc = parse_event_document(&event_doc_url(2_150_205), XC_STATE).expect("parses");
    write_schools(
        &store,
        &labels_of(&labelled_schools(&doc, UsJurisdiction::Iowa)),
    );
    let path = stage_capture(&dir, "event-doc-2150205.json", XC_STATE);
    let options = ResultOptions {
        documents: vec![path.clone()],
        ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
    };

    let report = collect(&context(&store, &fetcher), &options)
        .await
        .expect("the run completes");
    assert_eq!(report.adapter, SOURCE_ID);
    assert_eq!(report.rows, 136);
    assert_eq!(
        report.errors,
        0,
        "every row of this capture maps: {}",
        joined(&report)
    );
    assert_eq!(report.requests, 0, "the adapter fetches nothing");

    let performances: Vec<CanonicalPerformance> =
        store.scan(Table::Performances).expect("performances read");
    assert_eq!(performances.len(), 136);
    let winner = performances
        .iter()
        .find(|row| row.place == Some(1))
        .expect("one row is placed first");
    assert_eq!(winner.mark, Mark::TimeSeconds(CentiSeconds::new(110070)));
    assert!(
        winner.source_key.starts_with("athleticlive:2150205:"),
        "the performance key is the event's: {}",
        winner.source_key
    );
    assert_eq!(
        winner
            .evidence
            .iter()
            .map(|row| row.source.url.clone())
            .collect::<Vec<_>>(),
        vec![Some(event_doc_url(2_150_205))]
    );
    assert_eq!(winner.observed_grade.map(Grade::get), Some(12));

    let meets: Vec<CanonicalMeet> = store.scan(Table::Meets).expect("meets read");
    assert_eq!(meets.len(), 1);
    assert_eq!(
        meets[0].id.as_str(),
        state_meet().meet_id,
        "the harvest route's meet id"
    );
    assert_eq!(meets[0].date, "2025-10-31");
    assert_eq!(meets[0].state, Some(UsJurisdiction::Iowa));
    assert!(meets[0].source_identities.iter().any(|identity| {
        identity.id == STATE_MEET.to_string()
            && identity.namespace
                == SourceNamespace::TimerMeet {
                    provider: "live_results".to_string(),
                }
    }));

    let events: Vec<CanonicalEvent> = store.scan(Table::Events).expect("events read");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::CrossCountry);
    assert_eq!(
        events[0].round.as_deref(),
        Some("finals"),
        "`Finals` maps to the round marker"
    );
    assert_eq!(events[0].division, None);
    assert!(events[0]
        .source_labels
        .iter()
        .any(|label| label.label == "Run"));

    let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes).expect("athletes read");
    assert_eq!(athletes.len(), 136);
    assert_eq!(
        school_year_for_date(
            &meets[0].date,
            SchoolYear::new(2026).expect("2026 is a season")
        ),
        SchoolYear::containing(2025, 10).expect("2025-10 is a season")
    );
    assert!(athletes
        .iter()
        .any(|athlete| athlete.grad_year == GradYear::new(2026).expect("2026 is a cohort")));
    assert!(athletes
        .iter()
        .any(|athlete| athlete.grad_year == GradYear::new(2029).expect("2029 is a cohort")));

    let by_id: HashMap<&str, &CanonicalAthlete> = athletes
        .iter()
        .map(|athlete| (athlete.id.as_str(), athlete))
        .collect();
    let native = performances
        .iter()
        .filter(|performance| {
            performance.source_athlete.namespace == SourceNamespace::athletic_net("athlete")
        })
        .count();
    assert_eq!(native, 132, "owned through the provider athlete id");
    let mut row_scoped = 0usize;
    for performance in &performances {
        let athlete = by_id
            .get(performance.athlete.as_str())
            .unwrap_or_else(|| panic!("no athlete row for {}", performance.athlete.as_str()));
        assert_eq!(
            performance.source_athlete, athlete.source,
            "the owner is the athlete's own primary source: {:?}",
            performance.source_athlete
        );
        assert!(
            !performance.source_athlete.id.is_empty(),
            "the owner carries the provider's own id"
        );
        if performance.source_athlete.namespace
            == SourceNamespace::Other("athleticlive_result_row".to_string())
        {
            row_scoped += 1;
        }
    }
    assert_eq!(row_scoped, 4, "the rows that publish no athlete id");
}

#[tokio::test]
async fn a_result_pass_files_one_observation_per_athlete_id_the_capture_published() {
    let (dir, store, fetcher) = scratch();
    let doc = parse_event_document(&event_doc_url(2_150_205), XC_STATE).expect("parses");
    let labels = labelled_schools(&doc, UsJurisdiction::Iowa);
    write_schools(&store, &labels_of(&labels));
    let path = stage_capture(&dir, "event-doc-2150205.json", XC_STATE);
    let options = ResultOptions {
        documents: vec![path],
        ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
    };

    let report = collect(&context(&store, &fetcher), &options)
        .await
        .expect("the run completes");
    assert_eq!(report.errors, 0, "{}", joined(&report));

    let grade_number = |token: &str| match token {
        "FR" => Some(9),
        "SO" => Some(10),
        "JR" => Some(11),
        "SR" => Some(12),
        other => other.parse::<u8>().ok(),
    };
    let published: BTreeMap<u64, (&str, &str, u8)> = doc
        .rows
        .iter()
        .filter_map(|row| {
            let athlete = row.athlete.as_ref()?;
            let id = athlete
                .an_athlete_id
                .as_ref()
                .and_then(|value| value.as_u64())?;
            let name = athlete.name.as_deref()?;
            let school = athlete.team.as_ref().and_then(|team| team.school_name())?;
            let grade = athlete
                .grade
                .as_ref()
                .and_then(|value| value.as_str())
                .and_then(grade_number)?;
            Some((id, (name, school, grade)))
        })
        .collect();
    assert_eq!(
        published.len(),
        132,
        "the capture publishes 132 rows each carrying its own athlete id"
    );

    let observations: Vec<SourceObservation> = store
        .scan(Table::SourceObservations)
        .expect("observation log");
    let filed: BTreeMap<u64, &SourceAthleteObservation> = observations
        .iter()
        .filter_map(|row| match row {
            SourceObservation::Athlete(athlete) => {
                Some((athlete.source_athlete_id.parse().ok()?, athlete))
            }
            SourceObservation::School(_) => None,
        })
        .collect();
    assert_eq!(
        filed.keys().copied().collect::<Vec<_>>(),
        published.keys().copied().collect::<Vec<_>>(),
        "one observation per athlete id the capture published, keyed by the provider's own id"
    );

    let row_owned = observations
        .iter()
        .filter(|row| {
            row.namespace() == &SourceNamespace::Other("athleticlive_result_row".to_string())
        })
        .count();
    assert_eq!(row_owned, 4, "the rows that publish no athlete id");
    assert_eq!(
        observations.len(),
        136,
        "one observation per canonical athlete: 132 provider ids and four row-owned fallbacks"
    );

    let captured: Vec<&str> = labels.iter().map(|(_, name)| name.as_str()).collect();
    for (id, (name, school, grade)) in &published {
        let seen = filed.get(id).expect("every published id is filed");
        assert_eq!(
            seen.namespace,
            SourceNamespace::athletic_net("athlete"),
            "the observation files under the athlete's own primary source"
        );
        assert_eq!(seen.id, format!("athleticnet:athlete:{id}"));
        assert_eq!(
            seen.observed_name.as_str(),
            *name,
            "the name the capture spelled"
        );
        assert_eq!(
            seen.observed_school.as_deref(),
            Some(*school),
            "the school the capture placed the athlete at"
        );
        assert!(
            captured.contains(school),
            "the observed school is one the capture published: {school}"
        );
        assert_eq!(
            seen.observed_grade
                .as_ref()
                .map(|observed| observed.grade.get()),
            Some(*grade),
            "the class the capture's own token states"
        );
        assert_eq!(seen.gender, Gender::Girls);
        assert_eq!(
            seen.profile_url.as_deref(),
            Some(format!("https://www.athletic.net/athlete/{id}/track-and-field").as_str()),
            "the profile page Athletic.net's own athlete id derives"
        );
        assert_eq!(seen.observed_on, OBSERVED_ON);
        assert!(
            !seen.source_row_key.is_empty(),
            "the row states where it was read"
        );
    }
    let winner = filed
        .get(&17_327_390)
        .expect("the capture's first-placed row");
    assert_eq!(winner.observed_name, "McKenna Montgomery");
    assert_eq!(winner.observed_school.as_deref(), Some("Albia"));
}

#[tokio::test]
async fn club_labels_and_an_empty_grade_cell_are_refused_not_invented() {
    let (dir, store, fetcher) = scratch();
    write_schools(
        &store,
        &[(UsJurisdiction::Michigan, "East Kentwood High School")],
    );
    let path = stage_capture(&dir, "event-doc-2254280.json", HJ_MITS);
    let options = ResultOptions {
        documents: vec![path],
        ..ResultOptions::for_meet(mits_meet(), OBSERVED_ON)
    };

    let report = collect(&context(&store, &fetcher), &options)
        .await
        .expect("the run completes");
    assert_eq!(report.rows, 0, "no row names a consolidated school");

    let meets: Vec<CanonicalMeet> = store.scan(Table::Meets).expect("meets read");
    assert_eq!(meets.len(), 1);
    assert_eq!(meets[0].id.as_str(), mits_meet().meet_id);
    assert_eq!(meets[0].date, "2026-02-14");
    assert_eq!(meets[0].state, Some(UsJurisdiction::Michigan));
    let events: Vec<CanonicalEvent> = store.scan(Table::Events).expect("events read");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::HighJump);
    assert_eq!(events[0].division.as_deref(), Some("MITS"));
    assert!(store
        .scan::<CanonicalAthlete>(Table::Athletes)
        .expect("athletes read")
        .is_empty());
}
