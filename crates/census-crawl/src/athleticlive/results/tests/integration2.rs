use super::*;

#[test]
fn a_standings_capture_folds_into_the_event_that_published_its_run_key() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let (dir, store, fetcher) = scratch()?;
    write_schools(&store, &[(UsJurisdiction::Iowa, "Waukon")])?;
    let doc_path = stage_capture(&dir, "event-doc-2150205.json", XC_STATE)?;
    let standings_path = stage_capture(
        &dir,
        "live-run-standings-1-1.json",
        &standings_payload("Miriam Downing", "SO", "Waukon"),
    )?;
    let options = ResultOptions {
        documents: vec![doc_path.clone()],
        standings: vec![StandingsCapture {
            run_id: "1-1".to_string(),
            path: standings_path,
        }],
        ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
    };

    let report = collect(&context(&store, &fetcher)?, &options).await?;
    check!(eq; report.errors, 0, "{}", joined(&report));
    check!(eq; report.rows, 8);

    let performances: Vec<CanonicalPerformance> =
        store.scan(Table::Performances)?;
    check!(eq; performances.len(), 8);
    let miriam: Vec<_> = performances
        .iter()
        .filter(|row| row.place == Some(26))
        .collect();
    check!(eq;
        miriam.len(),
        2,
        "a standings row cannot name-merge into a native athlete"
    );
    check!(ne; miriam[0].source_athlete, miriam[1].source_athlete);
    check!(miriam
        .iter()
        .all(|row| row.mark == Mark::TimeSeconds(CentiSeconds::new(122570))));
    let sources: std::collections::BTreeSet<_> = miriam
        .iter()
        .flat_map(|row| {
            row.evidence
                .iter()
                .filter_map(|evidence| evidence.source.url.clone())
        })
        .collect();
    check!(eq;
        sources,
        std::collections::BTreeSet::from([
            event_doc_url(2_150_205),
            crate::athleticlive::wire::standings_url(STATE_MEET, "1-1").ok_or("valid run key")?,
        ])
    );
    let identities: Vec<String> = store
        .scan::<CanonicalAthlete>(Table::Athletes)
        ?
        .into_iter()
        .flat_map(|athlete| athlete.source.into_iter().chain(athlete.source_links))
        .map(|identity| identity.id)
        .collect();
    check!(
        !identities.contains(&"42660317".to_string()),
        "{identities:?}"
    );
    Ok(())
    })
}

#[test]
fn a_standings_capture_whose_run_key_no_document_published_is_refused() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let (dir, store, fetcher) = scratch()?;
    write_schools(
        &store,
        &[(UsJurisdiction::Michigan, "East Kentwood High School")],
    )?;
    let doc_path = stage_capture(&dir, "event-doc-2254280.json", HJ_MITS)?;
    let standings_path = stage_capture(
        &dir,
        "live-run-standings-9-9.json",
        &standings_payload("Miriam Downing", "SO", "East Kentwood High School"),
    )?;
    let options = ResultOptions {
        documents: vec![doc_path.clone()],
        standings: vec![StandingsCapture {
            run_id: "9-9".to_string(),
            path: standings_path.clone(),
        }],
        ..ResultOptions::for_meet(mits_meet(), OBSERVED_ON)
    };

    let report = collect(&context(&store, &fetcher)?, &options).await?;
    check!(eq; report.errors, 1, "{}", joined(&report));
    check!(
        joined(&report).contains(&format!(
            "capture refused: {standings_path}: no event document in this run published run key `9-9`"
        )),
        "{}",
        joined(&report)
    );
    Ok(())
    })
}

#[test]
fn a_second_run_resumes_the_capture_the_first_journaled() -> TestResult {
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
                documents: vec![path],
                ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
            };

            let first = collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; first.rows, 136);
            let second = collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; second.rows, 0, "the capture is not read twice");
            let notes = joined(&second);
            check!(
                notes.contains("captures already journaled by an earlier run: 1"),
                "{notes}"
            );
            check!(
        notes.contains("canonical entities: meets 1 events 0 teams 0 athletes 0 performances 0"),
        "a resumed run rewrites only the meet it files under, and reads no capture: {notes}"
    );
            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq; performances.len(), 136, "the tables are not appended twice");
            Ok(())
        })
}

#[test]
fn a_capture_whose_rows_never_landed_is_read_again_by_the_next_run() -> TestResult {
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

            let mut walk = Run::new(&context(&store, &fetcher)?, &state_meet(), &options)?;
            walk.read_captures(&options)?;
            check!(
                journal(&store)?.is_empty(),
                "the walk writes no entry of its own: {:?}",
                journal(&store)?
            );
            check!(
                store
                    .scan::<CanonicalPerformance>(Table::Performances)?
                    .is_empty(),
                "the walk writes no row of its own"
            );
            drop(walk);

            let report = collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; report.rows, 136, "the capture is a document of 136 rows");
            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq;
                performances.len(),
                136,
                "the capture's rows land on the second run, not skipped as read"
            );
            let meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
            check!(eq; meets.len(), 1, "the meet the run files under");
            check!(eq;
                journal(&store)?,
                std::collections::HashSet::from([path]),
                "one entry per capture read"
            );
            Ok(())
        })
}
