use super::*;
use crate::milesplit::{read_retained_owned_result, RetainedResultError};

mod attacks;

const BODY: &[u8] = include_bytes!("../../fixtures/troy_725218.json");

fn interpret(store: &Store, capture: FetchOutcome) -> TestResult {
    let dir = tempfile::tempdir()?;
    let fetcher = Fetcher::new(
        dir.path().join("offline"),
        None,
        std::time::Duration::ZERO,
        HashMap::new(),
        vec!["milesplit.com".into()],
    )?
    .with_offline(true);
    let ctx = AdapterContext {
        capture: crate::CaptureContext::LegacyMetadata,
        fetcher: &fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("school year")?,
        observed_on: "2099-01-01".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 1).ok_or("as-of date")?,
        recording: None,
    };
    let page = parse_owned_meet(&capture.body, 725218)?;
    record_outcome(
        &ctx,
        &source()?,
        &OwnedMeetOutcome::historical_fixture(capture, OwnedMeetVerdict::Parsed(page)),
    )?;
    Ok(())
}

fn prepared() -> TestResult<(tempfile::TempDir, Store, FetchOutcome, OwnedAcquisitionRef)> {
    let (dir, store, capture, reference) = fixture(BODY)?;
    interpret(&store, capture.clone())?;
    Ok((dir, store, capture, reference))
}

fn row_key(capture: &FetchOutcome) -> TestResult<String> {
    let row = parse_owned_meet(&capture.body, 725218)?
        .rows
        .into_iter()
        .next()
        .ok_or("first parsed row")?;
    let payload = serde_json::to_value(&row)?;
    Ok(format!(
        "row/725218/{}/data[0]/{}",
        capture.content_digest,
        census_domain::model::serialized_digest(&payload)?,
    ))
}

fn set_interpretation(store: &Store, key: &str, payload: &Value) -> TestResult {
    let mut batch = store.write_batch();
    batch.journal_done(OWNED_MEET_PHASE, key, payload)?;
    batch.commit()?;
    Ok(())
}

#[test]
fn retained_result_replays_writer_bytes_and_exact_interpretation_without_writes() -> TestResult {
    let (dir, store, capture, reference) = prepared()?;
    let before = store.journal_payloads(OWNED_MEET_PHASE)?;
    let view = store.snapshot();
    let result = read_retained_owned_result(&view, &reference, "data[0]", budget()?)?;
    check!(eq; result.snapshot_sequence(), view.sequence());
    check!(eq; result.row().result_id, 201782263);
    check!(eq; result.row().source_athlete.id, "14222592");
    check!(eq; result.row().locator, "data[0]");
    check!(eq; result.interpretation_key(), row_key(&capture)?);
    check!(eq; result.acquisition().body(), BODY);
    check!(eq; result.acquired_at().to_rfc3339(), "2026-10-01T23:44:16+00:00");
    check!(eq; store.journal_payloads(OWNED_MEET_PHASE)?, before);
    drop(view);
    drop(store);
    let reopened = Store::open(dir.path().join("store"))?;
    let replay =
        read_retained_owned_result(&reopened.snapshot(), &reference, "data[0]", budget()?)?;
    check!(eq; replay.row(), result.row());
    check!(eq; replay.interpretation_key(), result.interpretation_key());
    check!(eq; reopened.journal_payloads(OWNED_MEET_PHASE)?, before);
    Ok(())
}

#[test]
fn retained_result_requires_original_interpretation_even_when_raw_bytes_parse() -> TestResult {
    let (_dir, store, capture, reference) = fixture(BODY)?;
    let expected = row_key(&capture)?;
    check!(matches!(
        read_retained_owned_result(&store.snapshot(), &reference, "data[0]", budget()?),
        Err(RetainedResultError::MissingInterpretation { key }) if key == expected
    ));
    check!(store.journal_keys(OWNED_MEET_PHASE)?.is_empty());
    Ok(())
}

#[test]
fn retained_result_missing_rejected_and_malformed_locators_never_bind_another_row() -> TestResult {
    let (_dir, store, _capture, reference) = prepared()?;
    for locator in ["data[3]", "data[19999]"] {
        check!(matches!(
            read_retained_owned_result(&store.snapshot(), &reference, locator, budget()?),
            Err(RetainedResultError::LocatorMissing)
        ));
    }
    for locator in [
        "",
        "data[-1]",
        "data[00]",
        "data[01]",
        "data[20000]",
        "data[0]/fake",
        "data[+1]",
        "data[١]",
    ] {
        check!(matches!(
            read_retained_owned_result(&store.snapshot(), &reference, locator, budget()?),
            Err(RetainedResultError::InvalidLocator)
        ));
    }
    let mut body: Value = serde_json::from_slice(BODY)?;
    body["data"][0]["athleteId"] = json!("0");
    let bytes = serde_json::to_vec(&body)?;
    let capture = sample(&bytes, 200)?;
    let rejected = archive(&store, &capture)?;
    interpret(&store, capture)?;
    check!(matches!(
        read_retained_owned_result(&store.snapshot(), &rejected, "data[0]", budget()?),
        Err(RetainedResultError::LocatorMissing)
    ));
    Ok(())
}

#[test]
fn retained_result_exact_legacy_key_still_requires_replayed_capture_equality() -> TestResult {
    let (_dir, store, capture, reference) = fixture(BODY)?;
    let current_key = row_key(&capture)?;
    let (legacy, _) = current_key.rsplit_once('/').ok_or("row digest suffix")?;
    let row = parse_owned_meet(BODY, 725218)?
        .rows
        .into_iter()
        .next()
        .ok_or("first row")?;
    let expected = serde_json::to_value(&row)?;
    set_interpretation(&store, legacy, &expected)?;
    let result = read_retained_owned_result(&store.snapshot(), &reference, "data[0]", budget()?)?;
    check!(eq; result.interpretation_key(), legacy);
    check!(eq; result.row(), &row);
    let mut changed = expected;
    changed["team_id"] = json!(999);
    set_interpretation(&store, legacy, &changed)?;
    check!(matches!(
        read_retained_owned_result(&store.snapshot(), &reference, "data[0]", budget()?),
        Err(RetainedResultError::ChangedInterpretation { key }) if key == legacy
    ));
    Ok(())
}
