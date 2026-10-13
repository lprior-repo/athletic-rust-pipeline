use super::*;
use census_domain::model::EventKind;
use std::collections::BTreeMap;

const HISTORICAL_PHASE: &str = "milesplit_owned_meet_v3";

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

fn xc_body() -> TestResult<Vec<u8>> {
    let mut document: serde_json::Value =
        serde_json::from_slice(crate::milesplit::owned::tests::TROY)?;
    let rows = document["data"].as_array_mut().ok_or("owned rows")?;
    rows.truncate(1);
    let row = rows.first_mut().ok_or("owned row")?;
    row["eventName"] = serde_json::json!("Girls XC 5K");
    row["mark"] = serde_json::json!("20:25.70");
    Ok(serde_json::to_vec(&document)?)
}

fn seed_history(
    store: &Store,
    ctx: &AdapterContext<'_>,
    reference: &ResultSetRef,
    capture: FetchOutcome,
) -> TestResult {
    let mut page = match parse_owned_meet(&capture.body, 725218) {
        OwnedMeetVerdict::Parsed(page) => page,
        other => return Err(format!("XC owned fixture: {other:?}").into()),
    };
    check!(eq; page.rows.len(), 1);
    page.rows.first_mut().ok_or("owned row")?.event_kind = EventKind::Track5000m;
    let recording = crate::recording::Recording::new();
    let staged = AdapterContext {
        recording: Some(&recording),
        observed_on: ctx.observed_on.clone(),
        ..*ctx
    };
    record_outcome(
        &staged,
        reference,
        &OwnedMeetOutcome {
            capture,
            verdict: OwnedMeetVerdict::Parsed(page),
        },
    )?;
    let mut batch = store.write_batch();
    for entry in recording.drain().journal {
        let phase = if entry.phase == OWNED_MEET_PHASE {
            HISTORICAL_PHASE
        } else {
            &entry.phase
        };
        batch.journal_done(phase, &entry.key, &entry.payload)?;
    }
    batch.commit()?;
    Ok(())
}

#[test]
fn xc_interpretation_allocates_v4_and_preserves_v3_rows_and_v1_capture() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher, reference) = setup()?;
            let body = xc_body()?;
            seed(&fetcher, &reference, &body)?;
            let ctx = context(&store, &fetcher)?;
            let capture = fetch_owned_capture(&fetcher, &reference, &ctx.fetch_options()).await?;
            seed_history(&store, &ctx, &reference, capture)?;
            let history = snapshot(&store, HISTORICAL_PHASE)?;
            let raw = snapshot(&store, OWNED_CAPTURE_PHASE)?;
            check!(eq; OWNED_MEET_PHASE, "milesplit_owned_meet_v4");
            check!(eq; OWNED_CAPTURE_PHASE, "milesplit_owned_capture_v1");
            check!(eq; snapshot(&store, OWNED_MEET_PHASE)?, BTreeMap::new());
            check!(eq;
                history.values().find(|row| row["locator"] == "data[0]")
                    .ok_or("historical owned row")?["event_kind"],
                "track5000m"
            );
            let current = read_owned_meet(&ctx, &reference).await?;
            check!(eq; parsed(&current)?.rows.len(), 1);
            check!(eq; parsed(&current)?.rows.first().ok_or("current row")?.event_kind,
                EventKind::CrossCountry);
            check!(eq; parsed(&current)?.rows.first().ok_or("current row")?.result_id,
                201782263);
            let interpreted = snapshot(&store, OWNED_MEET_PHASE)?;
            check!(eq; interpreted.len(), 2);
            check!(eq; snapshot(&store, HISTORICAL_PHASE)?, history);
            check!(eq; snapshot(&store, OWNED_CAPTURE_PHASE)?, raw);
            drop(store);
            let store = Store::open(dir.path().join("store"))?;
            let replay = read_owned_meet(&context(&store, &fetcher)?, &reference).await?;
            check!(eq; replay.verdict, current.verdict);
            check!(eq; snapshot(&store, OWNED_MEET_PHASE)?, interpreted);
            check!(eq; snapshot(&store, HISTORICAL_PHASE)?, history);
            check!(eq; snapshot(&store, OWNED_CAPTURE_PHASE)?, raw);
            check!(eq; fetcher.stats().await.physical_requests(), 0);
            Ok(())
        })
}
