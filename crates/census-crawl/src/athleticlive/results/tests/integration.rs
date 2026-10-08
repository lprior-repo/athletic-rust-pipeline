use super::*;

#[test]
fn collect_maps_a_captured_state_final_into_the_canonical_tables() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher) = scratch()?;
            let doc = parse_event_document(&event_doc_url(2_150_205), XC_STATE)?;
            write_schools(
                &store,
                &labels_of(&labelled_schools(&doc, UsJurisdiction::Iowa)),
            )?;
            let path = stage_capture(&dir, "event-doc-2150205.json", XC_STATE)?;
            let options = ResultOptions {
                documents: vec![path.clone()],
                ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
            };

            let report = collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; report.adapter, SOURCE_ID);
            check!(eq; report.rows, 136);
            check!(eq;
                report.errors,
                0,
                "every row of this capture maps: {}",
                joined(&report)
            );
            check!(eq; report.requests, 0, "the adapter fetches nothing");

            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq; performances.len(), 136);
            let winner = performances
                .iter()
                .find(|row| row.place == Some(1))
                .ok_or("one row is placed first")?;
            check!(eq; winner.mark, Mark::TimeSeconds(ExactSeconds::parse("1100.7")?));
            check!(
                winner.source_key.starts_with("athleticlive:2150205:"),
                "the performance key is the event's: {}",
                winner.source_key
            );
            check!(eq;
                winner
                    .evidence
                    .iter()
                    .map(|row| row.source.url.clone())
                    .collect::<Vec<_>>(),
                vec![Some(event_doc_url(2_150_205))]
            );
            check!(eq; winner.observed_grade.map(Grade::get), Some(12));

            let meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
            check!(eq; meets.len(), 1);
            check!(eq;
                meets[0].id.as_str(),
                state_meet().meet_id,
                "the harvest route's meet id"
            );
            check!(eq; meets[0].date, "2025-10-31");
            check!(eq; meets[0].state, Some(UsJurisdiction::Iowa));
            check!(meets[0].source_identities.iter().any(|identity| {
                identity.id == STATE_MEET.to_string()
                    && identity.namespace
                        == SourceNamespace::TimerMeet {
                            provider: "live_results".to_string(),
                        }
            }));

            let events: Vec<CanonicalEvent> = store.scan(Table::Events)?;
            check!(eq; events.len(), 1);
            check!(eq; events[0].kind, EventKind::CrossCountry);
            check!(eq;
                events[0].round.as_deref(),
                Some("finals"),
                "`Finals` maps to the round marker"
            );
            check!(eq; events[0].division, None);
            check!(events[0]
                .source_labels
                .iter()
                .any(|label| label.label == "Run"));

            let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
            check!(eq; athletes.len(), 136);
            check!(eq;
                school_year_for_date(
                    &meets[0].date,
                    SchoolYear::new(2026).ok_or("2026 is a season")?
                ),
                SchoolYear::containing(2025, 10).ok_or("2025-10 is a season")?
            );
            let co2026 = GradYear::new(2026).ok_or("2026 is a cohort")?;
            let co2029 = GradYear::new(2029).ok_or("2029 is a cohort")?;
            check!(athletes.iter().any(|athlete| athlete.grad_year == co2026));
            check!(athletes.iter().any(|athlete| athlete.grad_year == co2029));

            let by_id: HashMap<&str, &CanonicalAthlete> = athletes
                .iter()
                .map(|athlete| (athlete.id.as_str(), athlete))
                .collect();
            let native = performances
                .iter()
                .filter(|performance| {
                    performance.source_athlete.as_ref().is_some_and(|identity| {
                        identity.namespace == SourceNamespace::athletic_net("athlete")
                    })
                })
                .count();
            check!(eq; native, 132, "owned through the provider athlete id");
            let mut row_scoped = 0usize;
            for performance in &performances {
                let athlete = by_id.get(performance.athlete.as_str()).ok_or_else(|| {
                    format!("no athlete row for {}", performance.athlete.as_str())
                })?;
                check!(eq;
                    performance.source_athlete, athlete.source,
                    "the owner is the athlete's own primary source: {:?}",
                    performance.source_athlete
                );
                let owner = performance
                    .source_athlete
                    .as_ref()
                    .ok_or("the owner is recorded")?;
                check!(
                    !owner.id.is_empty(),
                    "the owner carries the provider's own id"
                );
                if owner.namespace == SourceNamespace::Other("athleticlive_result_row".to_string())
                {
                    row_scoped += 1;
                }
            }
            check!(eq; row_scoped, 4, "the rows that publish no athlete id");
            Ok(())
        })
}

#[test]
fn a_result_pass_files_one_observation_per_athlete_id_the_capture_published() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let (dir, store, fetcher) = scratch()?;
    let doc = parse_event_document(&event_doc_url(2_150_205), XC_STATE)?;
    let labels = labelled_schools(&doc, UsJurisdiction::Iowa);
    write_schools(&store, &labels_of(&labels))?;
    let path = stage_capture(&dir, "event-doc-2150205.json", XC_STATE)?;
    let options = ResultOptions {
        documents: vec![path],
        ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
    };

    let report = collect(&context(&store, &fetcher)?, &options).await?;
    check!(eq; report.errors, 0, "{}", joined(&report));

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
    check!(eq;
        published.len(),
        132,
        "the capture publishes 132 rows each carrying its own athlete id"
    );

    let observations: Vec<SourceObservation> = store
        .scan(Table::SourceObservations)
        ?;
    let filed: BTreeMap<u64, &SourceAthleteObservation> = observations
        .iter()
        .filter_map(|row| match row {
            SourceObservation::Athlete(athlete) => {
                Some((athlete.source_athlete_id.parse().ok()?, athlete))
            }
            SourceObservation::School(_) => None,
        })
        .collect();
    check!(eq;
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
    check!(eq; row_owned, 4, "the rows that publish no athlete id");
    check!(eq;
        observations.len(),
        136,
        "one observation per canonical athlete: 132 provider ids and four row-owned fallbacks"
    );

    let captured: Vec<&str> = labels.iter().map(|(_, name)| name.as_str()).collect();
    for (id, (name, school, grade)) in &published {
        let seen = filed.get(id).ok_or("every published id is filed")?;
        check!(eq;
            seen.namespace,
            SourceNamespace::athletic_net("athlete"),
            "the observation files under the athlete's own primary source"
        );
        check!(eq; seen.id, format!("athleticnet:athlete:{id}"));
        check!(eq;
            seen.observed_name.as_str(),
            *name,
            "the name the capture spelled"
        );
        check!(eq;
            seen.observed_school.as_deref(),
            Some(*school),
            "the school the capture placed the athlete at"
        );
        check!(
            captured.contains(school),
            "the observed school is one the capture published: {school}"
        );
        check!(eq;
            seen.observed_grade
                .as_ref()
                .map(|observed| observed.grade.get()),
            Some(*grade),
            "the class the capture's own token states"
        );
        check!(eq; seen.gender, Gender::Girls);
        check!(eq;
            seen.profile_url.as_deref(),
            Some(format!("https://www.athletic.net/athlete/{id}/track-and-field").as_str()),
            "the profile page Athletic.net's own athlete id derives"
        );
    }
    let winner = filed
        .get(&17_327_390)
        .ok_or("the capture's first-placed row")?;
    check!(eq; winner.observed_name, "McKenna Montgomery");
    check!(eq; winner.observed_school.as_deref(), Some("Albia"));
    Ok(())
    })
}

#[test]
fn club_labels_and_an_empty_grade_cell_are_refused_not_invented() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher) = scratch()?;
            write_schools(
                &store,
                &[(UsJurisdiction::Michigan, "East Kentwood High School")],
            )?;
            let path = stage_capture(&dir, "event-doc-2254280.json", HJ_MITS)?;
            let options = ResultOptions {
                documents: vec![path],
                ..ResultOptions::for_meet(mits_meet(), OBSERVED_ON)
            };

            let report = collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; report.rows, 0, "no row names a consolidated school");

            let meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
            check!(eq; meets.len(), 1);
            check!(eq; meets[0].id.as_str(), mits_meet().meet_id);
            check!(eq; meets[0].date, "2026-02-14");
            check!(eq; meets[0].state, Some(UsJurisdiction::Michigan));
            let events: Vec<CanonicalEvent> = store.scan(Table::Events)?;
            check!(eq; events.len(), 1);
            check!(eq; events[0].kind, EventKind::HighJump);
            check!(eq; events[0].division.as_deref(), Some("MITS"));
            check!(store.scan::<CanonicalAthlete>(Table::Athletes)?.is_empty());
            Ok(())
        })
}
