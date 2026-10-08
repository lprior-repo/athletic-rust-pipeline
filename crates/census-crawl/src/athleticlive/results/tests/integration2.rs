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
    let time = ExactSeconds::parse("1225.7")?;
    check!(miriam
        .iter()
        .all(|row| row.mark == Mark::TimeSeconds(time)));
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
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
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
            check!(report
                .unfinished
                .iter()
                .any(|locator| locator == &standings_path));
            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(!performances
                .iter()
                .any(|row| row.evidence.iter().any(|evidence| evidence
                    .source
                    .url
                    .as_deref()
                    .is_some_and(|url| url.contains("9-9")))));
            Ok(())
        })
}

#[test]
fn unchanged_capture_replay_does_not_duplicate_performances() -> TestResult {
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
            check!(eq; second.rows, 0, "unchanged replay admits no additional rows");
            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq; performances.len(), 136, "the tables are not appended twice");
            Ok(())
        })
}

#[test]
fn partial_capture_replay_keeps_unfinished_and_commits_only_newly_resolved_rows() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher) = scratch()?;
            write_schools(&store, &[(UsJurisdiction::Iowa, "Waukon")])?;
            let path = stage_capture(&dir, "partial.json", XC_STATE)?;
            let options = ResultOptions {
                documents: vec![path.clone()],
                ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
            };
            let first = collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; first.rows, 7);
            check!(first.unfinished.contains(&path));
            check!(receipts(&store)?
                .values()
                .all(|receipt| receipt["complete"] == serde_json::json!(false)));
            let replay = collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; replay.rows, 0);
            check!(eq; replay.unfinished, first.unfinished);
            check!(eq; store.scan::<CanonicalPerformance>(Table::Performances)?.len(), 7);
            let doc = parse_event_document(&event_doc_url(2_150_205), XC_STATE)?;
            write_schools(
                &store,
                &labels_of(&labelled_schools(&doc, UsJurisdiction::Iowa)),
            )?;
            let completed = collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; completed.rows, 129);
            check!(!completed.unfinished.contains(&path));
            check!(receipts(&store)?
                .values()
                .any(|receipt| receipt["complete"] == serde_json::json!(true)));
            check!(eq; store.scan::<CanonicalPerformance>(Table::Performances)?.len(), 136);
            check!(eq; captures(&store)?.len(), 1);
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

            let recording = crate::recording::Recording::new();
            let mut staged = context(&store, &fetcher)?;
            staged.recording = Some(&recording);
            collect(&staged, &options).await?;
            check!(
                receipts(&store)?.is_empty() && captures(&store)?.is_empty(),
                "the walk writes neither a receipt nor an archived body of its own"
            );
            check!(
                store
                    .scan::<CanonicalPerformance>(Table::Performances)?
                    .is_empty(),
                "the walk writes no row of its own"
            );
            drop(recording);

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
            let receipts = receipts(&store)?;
            check!(eq; receipts.len(), 1, "one effect receipt per capture read");
            let (key, payload) = receipts.iter().next().ok_or("the receipt")?;
            check!(eq; payload["path"], serde_json::json!(path));
            check!(eq; payload["role"], serde_json::json!("event"));
            check!(eq; payload["meet"], serde_json::json!(STATE_MEET.to_string()));
            check!(eq; payload["parser"], serde_json::json!(super::super::PARSER));
            check!(eq;
                payload["digest"],
                serde_json::json!(content_digest(XC_STATE.as_bytes())),
                "the receipt binds the bytes it read"
            );
            check!(
                key.starts_with(&format!("{STATE_MEET}:event:")),
                "the receipt key names the meet and the role: {key}"
            );
            check!(eq;
                captures(&store)?,
                std::collections::HashSet::from([content_digest(XC_STATE.as_bytes())]),
                "the archived body is keyed by the digest it carries"
            );
            Ok(())
        })
}

#[test]
fn malformed_row_retains_later_neighbors_and_partial_replay_conserves_rows() -> TestResult {
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
            let mut document: serde_json::Value = serde_json::from_str(XC_STATE)?;
            *document
                .pointer_mut("/_source/r/0/m")
                .ok_or("published mark")? = serde_json::json!({"invalid":"mark"});
            let path = stage_capture(&dir, "malformed-row.json", &document.to_string())?;
            let options = ResultOptions {
                documents: vec![path.clone()],
                ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
            };
            let first = collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; first.rows, 135);
            check!(eq; first.errors, 1);
            check!(first.unfinished.contains(&path));
            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(performances
                .iter()
                .any(|performance| performance.source_key.contains(":row135:")));
            check!(!performances
                .iter()
                .any(|performance| performance.source_key.contains(":row0:")));
            let replay = collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; replay.rows, 0);
            check!(eq; replay.errors, 1);
            check!(replay.unfinished.contains(&path));
            check!(eq; store.scan::<CanonicalPerformance>(Table::Performances)?.len(), 135);
            check!(receipts(&store)?
                .values()
                .all(|receipt| receipt["complete"] == serde_json::json!(false)));
            Ok(())
        })
}
