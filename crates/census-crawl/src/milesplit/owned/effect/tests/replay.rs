use super::*;

fn mixed_body() -> TestResult<Vec<u8>> {
    let mut document: serde_json::Value =
        serde_json::from_slice(crate::milesplit::owned::tests::TROY)?;
    let relays: serde_json::Value =
        serde_json::from_slice(crate::milesplit::owned::tests::TEAM_RELAYS)?;
    document["data"]
        .as_array_mut()
        .ok_or("individual array")?
        .extend(
            relays["data"]
                .as_array()
                .ok_or("relay array")?
                .iter()
                .cloned(),
        );
    Ok(serde_json::to_vec(&document)?)
}

fn apply_journal(store: &Store, recorded: &crate::recording::Recorded) -> TestResult {
    let mut batch = store.write_batch();
    for entry in &recorded.journal {
        batch.journal_done(&entry.phase, &entry.key, &entry.payload)?;
    }
    batch.commit()?;
    Ok(())
}

fn assert_mixed_receipt(store: &Store) -> TestResult {
    let payloads = store.journal_payloads(OWNED_MEET_PHASE)?;
    let receipt = payloads
        .iter()
        .find(|row| row["disposition"] == "parsed")
        .ok_or("successful individual parse")?;
    check!(eq; receipt["team_relay_rows"], 3);
    check!(eq; receipt["rejected_individual_rows"], 0);
    check!(eq; receipt["individual_parse_complete"], true);
    check!(eq; receipt["ownership_complete"], false);
    check!(eq; receipt["completeness"], "unknown");
    for locator in ["data[3]", "data[4]", "data[5]"] {
        let relay = payloads
            .iter()
            .find(|row| row["locator"] == locator)
            .ok_or("retained relay reason")?;
        check!(eq; relay["kind"], "team_relay");
        check!(eq; relay.get("mark"), None);
        check!(eq; relay.get("source_athlete"), None);
    }
    check!(eq;
        payloads
            .iter()
            .filter_map(|row| row["result_id"].as_u64())
            .collect::<std::collections::BTreeSet<_>>(),
        std::collections::BTreeSet::from([201782263, 201782277, 201782806])
    );
    Ok(())
}

#[test]
fn relay_interpretation_replays_beside_unchanged_v1_history_and_capture_archive() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher, reference) = setup()?;
            let body = mixed_body()?;
            seed(&fetcher, &reference, &body)?;
            let original_ctx = context(&store, &fetcher)?;
            let capture =
                fetch_owned_capture(&fetcher, &reference, &original_ctx.fetch_options()).await?;
            let key = content_key(&reference, &capture);
            let prior =
                serde_json::json!({"disposition":"partial","owned_rows":3,"rejected_rows":3});
            let rejection = serde_json::json!({"locator":"data[3]","kind":"malformed_row",
        "detail":"invalid type: null, expected a borrowed string"});
            let mut batch = original_ctx.write_batch();
            record_capture(&mut batch, &key, &capture)?;
            batch.journal_done("milesplit_owned_meet_v1", &format!("partial/{key}"), &prior)?;
            batch.journal_done(
                "milesplit_owned_meet_v1",
                &format!("rejected/{key}/data[3]"),
                &rejection,
            )?;
            batch.commit()?;
            let originals = store.journal_payloads("milesplit_owned_meet_v1")?;
            let raw = store.journal_payloads(OWNED_CAPTURE_PHASE)?;
            let recording = crate::recording::Recording::new();
            let ctx = AdapterContext {
                recording: Some(&recording),
                ..context(&store, &fetcher)?
            };
            let outcome = read_owned_meet(&ctx, &reference).await?;
            check!(!parsed(&outcome)?.ownership_complete());
            let recorded = recording.drain();
            check!(eq; recorded.rows, Vec::new());
            check!(eq;
                store.journal_keys(OWNED_MEET_PHASE)?,
                std::collections::HashSet::new()
            );
            apply_journal(&store, &recorded)?;
            assert_mixed_receipt(&store)?;
            drop(store);
            let store = Store::open(dir.path().join("store"))?;
            assert_mixed_receipt(&store)?;
            let interpreted = store.journal_payloads(OWNED_MEET_PHASE)?;
            let replay = read_owned_meet(&context(&store, &fetcher)?, &reference).await?;
            check!(eq; replay.verdict, outcome.verdict);
            check!(eq;
                store.journal_payloads(OWNED_MEET_PHASE)?,
                interpreted
            );
            check!(eq;
                store
                    .journal_payloads("milesplit_owned_meet_v1")
                    ?,
                originals
            );
            let retained = store.journal_payloads(OWNED_CAPTURE_PHASE)?;
            check!(raw.iter().all(|original| retained.contains(original)));
            let chunk = raw
                .iter()
                .find(|row| row.get("raw_base64").is_some())
                .ok_or("raw capture")?;
            check!(eq;
                STANDARD
                    .decode(chunk["raw_base64"].as_str().ok_or("encoded")?)?,
                body
            );
            check!(eq; fetcher.stats().await.physical_requests(), 0);
            Ok(())
        })
}

#[test]
fn individual_owner_failure_remains_partial_without_attributing_relay_splits() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let mut document: serde_json::Value = serde_json::from_slice(&mixed_body()?)?;
            document["data"][0]["athleteId"] = serde_json::Value::Null;
            let body = serde_json::to_vec(&document)?;
            seed(&fetcher, &reference, &body)?;
            let outcome = read_owned_meet(&context(&store, &fetcher)?, &reference).await?;
            check!(!parsed(&outcome)?.individual_parse_complete());
            check!(eq;
                parsed(&outcome)?
                    .rows
                    .iter()
                    .map(|row| row.result_id)
                    .collect::<std::collections::BTreeSet<_>>(),
                std::collections::BTreeSet::from([201782277, 201782806])
            );
            let payloads = store.journal_payloads(OWNED_MEET_PHASE)?;
            let receipt = payloads
                .iter()
                .find(|row| row["disposition"] == "partial")
                .ok_or("partial receipt")?;
            check!(eq; receipt["team_relay_rows"], 3);
            check!(eq; receipt["rejected_individual_rows"], 1);
            check!(eq; receipt["ownership_complete"], false);
            check!(eq;
                payloads
                    .iter()
                    .find(|row| row["locator"] == "data[0]")
                    .ok_or("ownerless locator")?["kind"],
                "missing_owner"
            );
            Ok(())
        })
}
