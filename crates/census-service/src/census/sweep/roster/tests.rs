use super::refusal::retain_refusal;
use super::*;
use census_crawl::milesplit::{
    parse_roster, RosterQuarantine, RosterRejection, RosterRejectionKind, SourceRowLocator,
};
use census_crawl::net::FetchOutcome;
use census_domain::model::{CanonicalAthlete, CanonicalSchool, SchoolYear};
use census_domain::UsJurisdiction;
use census_store::Table;
use sha2::{Digest, Sha256};
use std::num::NonZeroU32;

mod windows;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn team() -> TeamRef {
    TeamRef {
        id: "52649".to_string(),
        slug: "example".to_string(),
        url: "https://wi.milesplit.com/teams/52649-example".to_string(),
        name: "Example High School".to_string(),
        city_state: "Madison, WI".to_string(),
    }
}

fn run() -> Result<RosterRun<'static>, Box<dyn std::error::Error>> {
    Ok(RosterRun {
        site: milesplit::Site::for_jurisdiction(UsJurisdiction::Wisconsin),
        observed_on: "2026-10-07",
        school_year: SchoolYear::DEFAULT,
        refresh: true,
        revision: NonZeroU32::new(1).ok_or("invalid fixture revision")?,
    })
}

fn body(second_year: &str) -> String {
    format!(
        r#"<link rel="canonical" href="{}/roster"><ul id="rosterDataset">{}{}</ul>"#,
        team().url,
        row("1", "Runner, Alice", "2027"),
        row("2", "Runner, Bob", second_year)
    )
}

fn row(id: &str, name: &str, year: &str) -> String {
    format!(
        r#"<li class="athlete-row data-row"><a href="https://wi.milesplit.com/athletes/{id}-runner">{name}</a><div class="column-gender">m</div><div class="column-grad-year">{year}</div><div data-season-id="2"><svg class="icon-yes"></svg></div></li>"#
    )
}

fn capture(body: String) -> Result<RosterOutcome, Box<dyn std::error::Error>> {
    let verdict = parse_roster(&body, team())?;
    Ok(RosterOutcome {
        verdict,
        capture: FetchOutcome {
            url: format!("{}/roster", team().url),
            response_url: None,
            method: "GET".to_string(),
            status: 200,
            content_digest: format!("{:x}", Sha256::digest(body.as_bytes())),
            bytes: body.len(),
            fetched_at: "2026-10-07T12:00:00Z".to_string(),
            from_cache: false,
            content_type: Some("text/html".to_string()),
            body: body.into_bytes(),
        },
    })
}

fn athletes(store: &Store) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut names = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::Athletes, |row: CanonicalAthlete| {
            names.push(row.canonical_name);
            Ok(())
        })?;
    names.sort();
    Ok(names)
}

#[tokio::test]
async fn partial_roster_survives_reopen_and_corrected_body_adds_only_missing_facts() -> TestResult {
    let root = tempfile::tempdir()?;
    let run = run()?;
    {
        let store = Store::open(root.path())?;
        apply(&store, &team(), &run, &capture(body(""))?).await?;
        let phase = rosters_phase(UsJurisdiction::Wisconsin, run.school_year, run.revision);
        let summary = summarize(
            &store,
            &phase,
            &[team()],
            UsJurisdiction::Wisconsin,
            run.school_year,
        )?;
        check!(eq; (summary.committed, summary.held, summary.athletes), (0, 1, 1));
        check!(eq; remaining(1, summary.committed, summary.held)?, 1);
        check!(eq; athletes(&store)?, vec!["Alice Runner".to_string()]);
        check!(eq; crate::census::scope::pending_rosters(&store, &[team()], UsJurisdiction::Wisconsin, run.school_year, run.revision)?, vec![team()]);
    }
    let store = Store::open(root.path())?;
    apply(&store, &team(), &run, &capture(body("2027"))?).await?;
    apply(&store, &team(), &run, &capture(body("2027"))?).await?;
    check!(eq; athletes(&store)?, vec!["Alice Runner".to_string(), "Bob Runner".to_string()]);
    let mut schools = Vec::new();
    store
        .snapshot()
        .for_each_observation(Table::Schools, |school: CanonicalSchool| {
            schools.push(school.name);
            Ok(())
        })?;
    check!(eq; schools, vec!["Example High School".to_string()]);
    check!(eq; crate::census::scope::pending_rosters(&store, &[team()], UsJurisdiction::Wisconsin, run.school_year, run.revision)?, Vec::<TeamRef>::new());
    Ok(())
}

#[tokio::test]
async fn failed_retry_and_quarantine_preserve_accepted_prefix_and_owed_counts() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let run = run()?;
    apply(&store, &team(), &run, &capture(body(""))?).await?;
    retain_refusal(
        &store,
        &team(),
        &run,
        &CrawlError::Invariant {
            detail: "capture unavailable".to_string(),
        },
    )?;
    let row = prior(&store, &team(), &run)?.ok_or("missing refusal journal")?;
    check!(eq; row.athletes, 1);
    check!(eq; row.disposition, census_crawl::CollectionDisposition::Failed);
    retain_refusal(
        &store,
        &team(),
        &run,
        &CrawlError::Fetch(census_crawl::net::FetchError::Http {
            status: 429,
            url: format!("{}/roster", team().url),
        }),
    )?;
    let blocked = prior(&store, &team(), &run)?.ok_or("missing blocked journal")?;
    check!(eq; (blocked.athletes, blocked.disposition), (1, census_crawl::CollectionDisposition::Blocked));
    let mut quarantined = capture(body(""))?;
    quarantined.verdict = milesplit::RosterVerdict::Quarantined {
        reason: RosterQuarantine::UnknownTemplate,
        rejected: Vec::new(),
        unfinished: None,
    };
    apply(&store, &team(), &run, &quarantined).await?;
    let row = prior(&store, &team(), &run)?.ok_or("missing quarantine journal")?;
    check!(eq; row.athletes, 1);
    check!(!row.is_terminal());
    check!(eq; athletes(&store)?, vec!["Alice Runner".to_string()]);
    let phase = rosters_phase(UsJurisdiction::Wisconsin, run.school_year, run.revision);
    let summary = summarize(
        &store,
        &phase,
        &[team()],
        UsJurisdiction::Wisconsin,
        run.school_year,
    )?;
    check!(eq; (summary.committed, summary.held, summary.athletes), (0, 1, 1));
    check!(eq; remaining(1, summary.committed, summary.held)?, 1);
    Ok(())
}

#[tokio::test]
async fn existing_application_receipt_does_not_pin_later_failed_progress() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let run = run()?;
    let corrected = capture(body("2027"))?;
    apply(&store, &team(), &run, &corrected).await?;
    retain_refusal(
        &store,
        &team(),
        &run,
        &CrawlError::Invariant {
            detail: "rebind pending".to_string(),
        },
    )?;
    apply(&store, &team(), &run, &corrected).await?;
    check!(prior(&store, &team(), &run)?
        .ok_or("missing corrected journal")?
        .is_terminal());
    check!(eq; athletes(&store)?, vec!["Alice Runner".to_string(), "Bob Runner".to_string()]);
    Ok(())
}

#[tokio::test]
async fn journal_without_explicit_completion_remains_resumable() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let run = run()?;
    apply(&store, &team(), &run, &capture(body("2027"))?).await?;
    let row = prior(&store, &team(), &run)?.ok_or("missing roster journal")?;
    let mut legacy = serde_json::to_value(row)?;
    legacy
        .as_object_mut()
        .ok_or("journal not object")?
        .remove("disposition");
    let phase = rosters_phase(UsJurisdiction::Wisconsin, run.school_year, run.revision);
    store.journal_done(&phase, "WI:52649", &legacy)?;
    check!(eq; crate::census::scope::pending_rosters(&store, &[team()], UsJurisdiction::Wisconsin, run.school_year, run.revision)?, vec![team()]);
    check!(eq; prior(&store, &team(), &run)?.ok_or("legacy absent")?.disposition, census_crawl::CollectionDisposition::Unknown);
    Ok(())
}

#[tokio::test]
async fn stopped_admission_records_a_durable_zero_prefix_that_stays_owed() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let run = run()?;
    let shared = super::super::shared(1)?;
    shared.lock().await.blocked = true;
    let fetcher = census_crawl::net::Fetcher::new(
        root.path().join("http"),
        None,
        std::time::Duration::ZERO,
        std::collections::HashMap::new(),
        Vec::new(),
    )?;
    let mut stopped = team();
    stopped.url = "http://127.0.0.1:1/teams/52649-example".to_string();
    super::super::units::roster_unit(&fetcher, &store, &shared, &stopped, &run).await;
    let row = prior(&store, &stopped, &run)?.ok_or("missing stopped journal")?;
    check!(!row.is_terminal());
    check!(row.refusal.is_some());
    check!(row.capture.is_none());
    check!(eq; row.athletes, 0);
    check!(eq; shared.lock().await.blocked_skipped, 1);
    let phase = rosters_phase(UsJurisdiction::Wisconsin, run.school_year, run.revision);
    let summary = summarize(
        &store,
        &phase,
        &[stopped.clone(), stopped],
        UsJurisdiction::Wisconsin,
        run.school_year,
    )?;
    check!(eq; (summary.total, summary.committed, summary.held), (1, 0, 1));
    check!(eq; remaining(summary.total, summary.committed, summary.held)?, 1);
    Ok(())
}

#[tokio::test]
async fn completion_requires_scoped_capture_without_refusal_quarantine_or_rejections() -> TestResult
{
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let run = run()?;
    let phase = rosters_phase(UsJurisdiction::Wisconsin, run.school_year, run.revision);
    check!(eq; crate::census::scope::pending_rosters(&store, &[team()], UsJurisdiction::Wisconsin, run.school_year, run.revision)?, vec![team()]);
    apply(&store, &team(), &run, &capture(body("2027"))?).await?;
    let complete = prior(&store, &team(), &run)?.ok_or("missing completion")?;
    let mut payload = serde_json::to_value(complete)?;
    for (field, value) in [
        ("refusal", serde_json::json!("source refused")),
        (
            "quarantine",
            serde_json::to_value(RosterQuarantine::NoReadableRows)?,
        ),
        (
            "rejected",
            serde_json::to_value(vec![RosterRejection {
                row: SourceRowLocator {
                    ordinal: 4,
                    byte_offset: 128,
                    byte_length: 32,
                },
                athlete_id: None,
                kind: RosterRejectionKind::MissingName,
            }])?,
        ),
        ("capture", serde_json::Value::Null),
        (
            "year",
            serde_json::json!(run
                .school_year
                .get()
                .checked_sub(1)
                .ok_or("fixture year underflow")?),
        ),
    ] {
        let previous = payload
            .as_object_mut()
            .ok_or("journal not object")?
            .insert(field.to_string(), value);
        store.journal_done(&phase, "WI:52649", &payload)?;
        check!(eq; crate::census::scope::pending_rosters(&store, &[team()], UsJurisdiction::Wisconsin, run.school_year, run.revision)?, vec![team()]);
        match previous {
            Some(previous) => {
                payload
                    .as_object_mut()
                    .ok_or("journal not object")?
                    .insert(field.to_string(), previous);
            }
            None => {
                payload
                    .as_object_mut()
                    .ok_or("journal not object")?
                    .remove(field);
            }
        }
    }
    store.journal_done(&phase, "WI:52649", &payload)?;
    check!(eq; crate::census::scope::pending_rosters(&store, &[team()], UsJurisdiction::Wisconsin, run.school_year, run.revision)?, Vec::<TeamRef>::new());
    check!(eq; crate::census::scope::pending_rosters(&store, &[team()], UsJurisdiction::Wisconsin, run.school_year, NonZeroU32::new(2).ok_or("invalid revision")?)?, vec![team()]);
    Ok(())
}
