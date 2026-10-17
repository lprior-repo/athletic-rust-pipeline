use super::*;
use census_store::recorded::Recorded;
use std::collections::BTreeMap;

const HISTORICAL_PHASE: &str = "milesplit_result_sets_v5";

fn snapshot(store: &Store, phase: &str) -> TestResult<BTreeMap<String, serde_json::Value>> {
    store
        .journal_keys(phase)?
        .into_iter()
        .map(|key| {
            let payload = store
                .journal_payload(phase, &key)?
                .ok_or("receipt payload")?;
            Ok((key, payload))
        })
        .collect()
}

fn historical_completion(
    store: &Store,
    reference: &ResultSetRef,
    recorded: &Recorded,
) -> TestResult<String> {
    let mut payload = recorded
        .journal
        .iter()
        .find(|entry| {
            entry.phase == super::super::super::RESULT_SET_PHASE
                && entry.payload["disposition"] == "projection_applied"
        })
        .ok_or("completed result projection")?
        .payload
        .clone();
    payload["parser_revision"] = json!("milesplit_owned_v3_raw_metadata_v1");
    payload["schema_revision"] = json!(HISTORICAL_PHASE);
    let key = super::super::receipt::identified(reference, &payload)?;
    let mut batch = store.write_batch();
    for entry in &recorded.journal {
        if entry.phase == crate::milesplit::OWNED_CAPTURE_PHASE {
            batch.journal_done(&entry.phase, &entry.key, &entry.payload)?;
        } else if entry.phase == crate::milesplit::OWNED_MEET_PHASE {
            batch.journal_done("milesplit_owned_meet_v3", &entry.key, &entry.payload)?;
        }
    }
    batch.journal_done(HISTORICAL_PHASE, &key, &payload)?;
    batch.commit()?;
    Ok(key)
}

#[test]
fn v5_completion_cannot_authorize_or_suppress_v6_projection_of_unchanged_captures() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher, reference) = setup()?;
            seed_owned(&fetcher, &reference, TROY)?;
            seed_metadata(&fetcher, &reference)?;
            let recording = crate::recording::Recording::new();
            let staged = AdapterContext {
                recording: Some(&recording),
                ..context(&store, &fetcher)?
            };
            let drafted = super::super::super::collect(&staged, &options(&reference)).await?;
            check!(eq; drafted.errors, 0);
            let recorded = recording.drain();
            let old_key = historical_completion(&store, &reference, &recorded)?;
            let history = snapshot(&store, HISTORICAL_PHASE)?;
            let raw = snapshot(&store, crate::milesplit::OWNED_CAPTURE_PHASE)?;
            let interpreted = snapshot(&store, "milesplit_owned_meet_v3")?;
            check!(eq; super::super::super::RESULT_SET_PHASE, "milesplit_result_sets_v6");
            check!(eq; super::super::receipt::PARSER_REVISION, "milesplit_owned_v4_raw_metadata_v1");
            check!(eq; snapshot(&store, super::super::super::RESULT_SET_PHASE)?, BTreeMap::new());
            let applied = super::super::super::collect(
                &context(&store, &fetcher)?, &options(&reference),
            ).await?;
            check!(eq; applied.errors, 0, "{:?}", applied.notes);
            check!(eq; store.walk_table(Table::Performances)?.rows, 3);
            let current = snapshot(&store, super::super::super::RESULT_SET_PHASE)?;
            let (new_key, completed) = current.iter()
                .find(|(_, payload)| payload["disposition"] == "projection_applied")
                .ok_or("new result projection completion")?;
            check!(ne; new_key, &old_key);
            check!(eq; completed["parser_revision"], "milesplit_owned_v4_raw_metadata_v1");
            check!(eq; completed["schema_revision"], "milesplit_result_sets_v6");
            check!(eq; snapshot(&store, HISTORICAL_PHASE)?, history);
            check!(eq; snapshot(&store, crate::milesplit::OWNED_CAPTURE_PHASE)?, raw);
            check!(eq; snapshot(&store, "milesplit_owned_meet_v3")?, interpreted);
            let physical = replay::physical(&store)?;
            drop(store);
            let store = Store::open(dir.path().join("store"))?;
            let repeated = super::super::super::collect(
                &context(&store, &fetcher)?, &options(&reference),
            ).await?;
            check!(eq; repeated.errors, 0);
            check!(eq; replay::physical(&store)?, physical);
            check!(eq; snapshot(&store, super::super::super::RESULT_SET_PHASE)?, current);
            check!(eq; snapshot(&store, HISTORICAL_PHASE)?, history);
            check!(eq; snapshot(&store, crate::milesplit::OWNED_CAPTURE_PHASE)?, raw);
            check!(eq; snapshot(&store, "milesplit_owned_meet_v3")?, interpreted);
            check!(eq; fetcher.stats().await.physical_requests(), 0);
            Ok(())
        })
}

#[test]
fn current_revision_qualifies_owned_xc_event_code_before_committing_completion() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let mut document: serde_json::Value = serde_json::from_slice(TROY)?;
            let rows = document["data"].as_array_mut().ok_or("owned rows")?;
            rows.truncate(1);
            let row = rows.first_mut().ok_or("owned row")?;
            row["eventName"] = json!("Cross Country");
            row["eventCode"] = json!("Girls XC 5K");
            row["mark"] = json!("20:25.70");
            seed_owned(&fetcher, &reference, &serde_json::to_vec(&document)?)?;
            seed_metadata(&fetcher, &reference)?;
            let report =
                super::super::super::collect(&context(&store, &fetcher)?, &options(&reference))
                    .await?;
            check!(eq; report.errors, 0, "{:?}", report.notes);
            check!(eq; store.walk_table(Table::Performances)?.rows, 1);
            let events: Vec<census_domain::model::CanonicalEvent> = store.scan(Table::Events)?;
            check!(eq; events.len(), 1);
            let event = events.first().ok_or("owned XC event")?;
            check!(eq; event.kind, census_domain::model::EventKind::CrossCountry);
            check!(eq; event.source_labels.iter().map(|label| label.label.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
            std::collections::BTreeSet::from(["Cross Country", "Girls XC 5K"]));
            let current = snapshot(&store, super::super::super::RESULT_SET_PHASE)?;
            let completed = current
                .values()
                .find(|payload| payload["disposition"] == "projection_applied")
                .ok_or("XC result projection completion")?;
            check!(eq; completed["parser_revision"], "milesplit_owned_v4_raw_metadata_v1");
            check!(eq; completed["schema_revision"], "milesplit_result_sets_v6");
            check!(eq; fetcher.stats().await.physical_requests(), 0);
            Ok(())
        })
}
