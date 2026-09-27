use super::*;

#[tokio::test]
async fn the_event_summary_lists_individual_events_and_excludes_relays() {
    let summary = r#"{"a":{"i":2254280,"ec":"Individual","rui":"19-1","ab":"HJ","un":"High Jump","gl":"Girls"},
            "b":{"i":999001,"ec":"Relay","rui":"7-1","peb":"Relay","ab":"4x400m"},
            "c":{"i":999002,"ec":"Individual","ab":"Underwater Basket Weaving"}}"#.to_string();
    let events =
        parse_event_summary(&event_summary_url(MITS_MEET), &summary).expect("the summary parses");
    assert_eq!(events.len(), 3);
    assert_eq!(events.iter().filter(|event| event.is_relay()).count(), 1);
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(event.kind(), EventKind::Unmapped { .. }))
            .count(),
        1
    );
    assert_eq!(events[0].event_id(), Some(2_254_280));

    let (dir, store, fetcher) = scratch();
    write_schools(
        &store,
        &[(UsJurisdiction::Michigan, "East Kentwood High School")],
    );
    let summary_path = stage_capture(&dir, "event-summary-61710.json", &summary);
    let doc_path = stage_capture(&dir, "event-doc-2254280.json", HJ_MITS);
    let options = ResultOptions {
        summary: Some(summary_path),
        documents: vec![doc_path],
        ..ResultOptions::for_meet(mits_meet(), OBSERVED_ON)
    };
    let report = collect(&context(&store, &fetcher), &options)
        .await
        .expect("the run completes");
    let notes = joined(&report);
    assert!(
        notes.contains("events listed 3 (relay 1, unmapped 1, unfetched 1)"),
        "the relay is counted and never fetched, and the event with no document is reported: {notes}"
    );
}

#[tokio::test]
async fn a_manifest_imports_every_meet_it_names_and_lands_on_the_harvest_ids() {
    let (dir, store, fetcher) = scratch();
    let doc = parse_event_document(&event_doc_url(2_150_205), XC_STATE).expect("parses");
    write_schools(
        &store,
        &labels_of(&labelled_schools(&doc, UsJurisdiction::Iowa)),
    );
    let xc_path = stage_capture(&dir, "event-doc-2150205.json", XC_STATE);
    let hj_path = stage_capture(&dir, "event-doc-2254280.json", HJ_MITS);
    let manifest = serde_json::json!({
        "meets": [
            {
                "athleticlive_meet_id": STATE_MEET,
                "tenant": "live_results",
                "name": "Iowa High School State Championships",
                "state": "IA",
                "date": "2025-10-31",
                "documents": [xc_path],
            },
            {
                "athleticlive_meet_id": MITS_MEET,
                "tenant": "live_results",
                "name": "MITS 4",
                "state": "Michigan",
                "date": "2026-02-14",
                "documents": [hj_path],
            },
            {
                "athleticlive_meet_id": STATE_MEET,
                "tenant": "live_results",
                "name": "Iowa High School State Championships",
                "state": "IA",
                "date": "2025-10-31",
                "documents": [xc_path],
            }
        ]
    });
    let manifest_path = stage_capture(&dir, "manifest.json", &manifest.to_string());
    let options = ManifestOptions {
        input: Some(manifest_path),
        limit: None,
        observed_on: OBSERVED_ON.to_string(),
        states: Vec::new(),
    };

    let report = collect_manifest(&context(&store, &fetcher), &options)
        .await
        .expect("the import completes");
    assert_eq!(report.adapter, SOURCE_ID);
    assert_eq!(
        report.rows, 136,
        "the state final's 136 rows map, and the field event's club-labelled rows map none"
    );
    assert_eq!(report.errors, 0, "{}", joined(&report));
    assert_eq!(report.requests, 0, "the adapter fetches nothing");

    let journaled = journal(&store);
    assert!(
        journaled.contains(&xc_path) && journaled.contains(&hj_path),
        "the manifest's own documents are the captures read: {journaled:?}"
    );

    let meets: Vec<CanonicalMeet> = store.scan(Table::Meets).expect("meets read");
    assert_eq!(meets.len(), 2, "one canonical meet per manifest entry");
    for expected in [state_meet().meet_id, mits_meet().meet_id] {
        assert!(
            meets.iter().any(|meet| meet.id.as_str() == expected),
            "the harvest id {expected} is the one results landed on"
        );
    }

    let performances: Vec<CanonicalPerformance> =
        store.scan(Table::Performances).expect("performances read");
    assert_eq!(
        performances.len(),
        136,
        "the repeated meet entry lands no second copy of the state final"
    );
    assert!(
        performances
            .iter()
            .all(|row| row.source_key.starts_with("athleticlive:2150205:")),
        "only the state final contributes performances"
    );

    let again = collect_manifest(&context(&store, &fetcher), &options)
        .await
        .expect("the repeated import completes");
    assert_eq!(again.rows, 0, "the first import journaled every capture");
    assert_eq!(again.errors, 0, "{}", joined(&again));
    let appended: Vec<CanonicalPerformance> =
        store.scan(Table::Performances).expect("performances read");
    assert_eq!(appended.len(), 136, "a repeated import appends no row");
}

#[tokio::test]
async fn a_manifest_that_places_no_jurisdiction_fails_by_name() {
    let body = r#"{"meets":[{"athleticlive_meet_id":1,"tenant":"t","name":"n","state":"Atlantis","date":"2026-01-01"}]}"#;
    let error =
        super::super::manifest::parse_manifest(body, OBSERVED_ON).expect_err("no jurisdiction");
    assert!(
        error.to_string().contains("Atlantis"),
        "the refusal names the unknown jurisdiction: {error}"
    );
}

#[tokio::test]
async fn the_results_route_requires_a_manifest() {
    let (_dir, store, fetcher) = scratch();
    let error = collect_manifest(
        &context(&store, &fetcher),
        &ManifestOptions {
            input: None,
            limit: None,
            observed_on: OBSERVED_ON.to_string(),
            states: Vec::new(),
        },
    )
    .await
    .expect_err("a manifest is required");
    assert!(
        error.to_string().contains("requires --input"),
        "the refusal names the flag: {error}"
    );
}
