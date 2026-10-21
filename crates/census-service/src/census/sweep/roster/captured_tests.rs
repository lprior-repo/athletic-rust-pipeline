use super::*;
use census_crawl::milesplit::RosterQuarantine;
use census_crawl::net::FetchOutcome;
use census_domain::model::{CanonicalAthlete, SchoolYear};
use census_domain::UsJurisdiction;
use census_store::Table;
use sha2::{Digest, Sha256};
use std::num::NonZeroU32;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn team() -> TeamRef {
    TeamRef {
        id: "52649".into(),
        slug: "example".into(),
        url: "https://wi.milesplit.com/teams/52649-example".into(),
        name: "Example High School".into(),
        city_state: "Madison, WI".into(),
    }
}

fn run() -> TestResult<RosterRun<'static>> {
    Ok(RosterRun {
        site: milesplit::Site::for_jurisdiction(UsJurisdiction::Wisconsin),
        observed_on: "2026-10-07",
        school_year: SchoolYear::DEFAULT,
        refresh: true,
        revision: NonZeroU32::new(1).ok_or("fixture revision is zero")?,
    })
}

fn body(second_year: &str) -> String {
    format!(
        "<link rel=\"canonical\" href=\"{}/roster\"><ul id=\"rosterDataset\">{}{}</ul>",
        team().url,
        row("1", "Runner, Alice", "2027"),
        row("2", "Runner, Bob", second_year)
    )
}

fn row(id: &str, name: &str, year: &str) -> String {
    format!(
        "<li class=\"athlete-row data-row\"><a href=\"https://wi.milesplit.com/athletes/{id}-runner\">{name}</a><div class=\"column-gender\">m</div><div class=\"column-grad-year\">{year}</div></li>"
    )
}

fn captured(status: u16, body: &[u8]) -> TestResult<RosterOutcome> {
    Ok(RosterOutcome::from_capture(
        &team(),
        FetchOutcome {
            url: format!("{}/roster", team().url),
            response_url: Some(format!("{}/roster", team().url)),
            method: "GET".into(),
            status,
            content_digest: format!("{:x}", Sha256::digest(body)),
            bytes: body.len(),
            fetched_at: "2026-10-07T12:00:00Z".into(),
            from_cache: true,
            content_type: Some("text/html".into()),
            body: body.to_vec(),
        },
    )?)
}

fn open(root: &std::path::Path) -> TestResult<Store> {
    let (store, writer) = Store::open_current_qualification_v3(root)?.into_parts();
    drop(writer);
    Ok(store)
}

fn witness(store: &Store, run: &RosterRun<'_>) -> TestResult<(String, u64, serde_json::Value)> {
    Ok((
        store.snapshot().tables_digest(&Table::ALL)?,
        store.receipt_count()?,
        serde_json::to_value(prior(store, &team(), run)?.ok_or("roster journal is absent")?)?,
    ))
}

#[tokio::test]
async fn accepted_captured_roster_reuses_original_apply_receipt_and_cold_projection() -> TestResult
{
    for (year, expected_members, complete) in [("2027", 2, true), ("", 1, false)] {
        let root = tempfile::tempdir()?;
        Store::create_current_qualification_v3(root.path())?;
        let run = run()?;
        let read = captured(200, body(year).as_bytes())?;
        let held = {
            let store = open(root.path())?;
            let first = accept_captured(&store, &team(), &run, &read).await?;
            check!(first.written());
            check!(eq; store.scan::<CanonicalAthlete>(Table::Athletes)?.len(), expected_members);
            let journal = prior(&store, &team(), &run)?.ok_or("captured roster journal")?;
            check!(eq; journal.is_terminal(), complete);
            check!(eq; journal.rejected.len(), usize::from(!complete));
            check!(eq; journal.capture.ok_or("journal capture")?.content_digest, read.capture.content_digest);
            let before = witness(&store, &run)?;
            let repeated = apply(&store, &team(), &run, &read).await?;
            check!(repeated.repeated());
            check!(eq; repeated.receipt(), first.receipt());
            check!(eq; witness(&store, &run)?, before);
            before
        };
        let store = open(root.path())?;
        check!(eq; witness(&store, &run)?, held);
        check!(eq; store.scan::<CanonicalAthlete>(Table::Athletes)?.len(), expected_members);
    }
    Ok(())
}

#[tokio::test]
async fn captured_adapter_retains_the_same_exact_metadata_refusal_as_original_apply() -> TestResult
{
    let root = tempfile::tempdir()?;
    Store::create_current_qualification_v3(root.path())?;
    let store = open(root.path())?;
    let run = run()?;
    let mut read = captured(200, body("2027").as_bytes())?;
    read.capture.content_digest = "0".repeat(64);
    let failure = accept_captured(&store, &team(), &run, &read)
        .await
        .err()
        .ok_or("inconsistent metadata was accepted")?;
    check!(matches!(failure.primary(), CrawlError::Invariant { detail }
        if detail == "roster capture metadata is inconsistent"));
    check!(eq; store.scan::<CanonicalAthlete>(Table::Athletes)?.len(), 0);
    let row = prior(&store, &team(), &run)?.ok_or("retained capture refusal")?;
    check!(eq; row.disposition, census_crawl::CollectionDisposition::Failed);
    let reason = failure.to_string();
    check!(eq; row.refusal.as_deref(), Some(reason.as_str()));
    check!(eq; row.capture.ok_or("failed capture metadata")?.content_digest, read.capture.content_digest);
    let held = witness(&store, &run)?;
    let original = apply(&store, &team(), &run, &read)
        .await
        .err()
        .ok_or("original apply accepted inconsistent metadata")?;
    check!(eq; original.to_string(), failure.to_string());
    check!(eq; witness(&store, &run)?, held);
    Ok(())
}

#[tokio::test]
async fn not_found_and_invalid_encoding_keep_original_quarantine_receipt_without_members(
) -> TestResult {
    for (status, reason) in [
        (404, RosterQuarantine::NotFound),
        (200, RosterQuarantine::InvalidEncoding),
    ] {
        let root = tempfile::tempdir()?;
        Store::create_current_qualification_v3(root.path())?;
        let run = run()?;
        let read = captured(status, &[255, 254])?;
        let held = {
            let store = open(root.path())?;
            let first = accept_captured(&store, &team(), &run, &read).await?;
            check!(first.written());
            check!(eq; store.scan::<CanonicalAthlete>(Table::Athletes)?.len(), 0);
            let row = prior(&store, &team(), &run)?.ok_or("quarantine journal")?;
            check!(eq; row.quarantine, Some(reason));
            check!(eq; row.disposition, census_crawl::CollectionDisposition::Quarantined);
            check!(!row.is_terminal());
            check!(eq; row.capture.ok_or("quarantined capture metadata")?.status, status);
            let held = witness(&store, &run)?;
            let repeated = apply(&store, &team(), &run, &read).await?;
            check!(repeated.repeated());
            check!(eq; repeated.receipt(), first.receipt());
            check!(eq; witness(&store, &run)?, held);
            held
        };
        let store = open(root.path())?;
        check!(eq; witness(&store, &run)?, held);
    }
    Ok(())
}
