use super::*;
use crate::net::cache::content_digest;
use crate::net::Fetcher;
use census_domain::model::SchoolYear;
use census_store::Store;
use std::collections::HashMap;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod uncaptured;

fn setup() -> TestResult<(tempfile::TempDir, Store, Fetcher, ResultSetRef)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::ZERO,
        HashMap::new(),
        vec!["milesplit.com".into()],
    )?
    .with_offline(true);
    let reference =
        ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266814/raw")
            .ok_or("source reference")?;
    Ok((dir, store, fetcher, reference))
}

fn context<'a>(
    store: &'a Store,
    fetcher: &'a Fetcher,
    capture: &'a crate::ResultsCaptureContext,
) -> TestResult<AdapterContext<'a>> {
    Ok(AdapterContext {
        capture: crate::CaptureContext::DurableV1(capture),
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("school year")?,
        observed_on: "2026-10-01".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 1).ok_or("snapshot date")?,
        recording: None,
    })
}

#[test]
fn malformed_partial_and_refused_captures_never_receive_parse_success_receipts() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let durable = crate::net::durable_capture_context(&_dir.path().join("capture"))?;
            let ctx = context(&store, &fetcher, &durable)?;
            let mut partial: serde_json::Value =
                serde_json::from_slice(crate::milesplit::owned::tests::TROY)?;
            partial["data"][0]["athleteId"] = serde_json::json!("0");
            let partial = serde_json::to_vec(&partial)?;
            for (status, body) in [
                (200, b"<html>Forbidden</html>".as_slice()),
                (200, partial.as_slice()),
                (404, b"not found".as_slice()),
            ] {
                let capture = FetchOutcome {
                    url: owned_meet_url(&reference)?,
                    response_url: None,
                    method: "GET".into(),
                    status,
                    content_digest: content_digest(body),
                    bytes: body.len(),
                    fetched_at: "2026-10-01T23:44:16Z".into(),
                    from_cache: false,
                    content_type: None,
                    body: body.to_vec(),
                };
                let verdict = if status == 200 {
                    match parse_owned_meet(body, 725218) {
                        Ok(page) => OwnedMeetVerdict::Parsed(page),
                        Err(OwnedMeetParseError::Malformed(failure)) => {
                            OwnedMeetVerdict::Malformed {
                                detail: failure.to_string(),
                            }
                        }
                        Err(OwnedMeetParseError::Resource(failure)) => return Err(failure.into()),
                    }
                } else {
                    OwnedMeetVerdict::Refused { status }
                };
                record_outcome(
                    &ctx,
                    &reference,
                    &OwnedMeetOutcome::historical_fixture(capture, verdict),
                )?;
                let digest = content_digest(body);
                let summaries = store.journal_payloads(OWNED_MEET_PHASE)?;
                let summary = summaries
                    .iter()
                    .find(|value| value["capture"]["content_digest"] == digest)
                    .ok_or("retained interpretation of the rejected or partial capture")?;
                check!(ne; summary["individual_parse_complete"], true);
                check!(store
                    .journal_keys(OWNED_CAPTURE_PHASE)?
                    .contains(&format!("725218/{digest}/manifest")));
            }
            let rows = store.journal_payloads(OWNED_MEET_PHASE)?;
            check!(rows.iter().any(|row| row["result_id"] == 201782277));
            check!(rows
                .iter()
                .any(|row| row["locator"] == "data[0]" && row["kind"] == "invalid_identity"));
            Ok(())
        })
}
