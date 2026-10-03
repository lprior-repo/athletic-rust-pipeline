#[path = "tests/acquisition_regressions.rs"]
mod acquisition_regressions;
#[path = "tests/assertions.rs"]
mod assertions;
#[path = "tests/http_fixture.rs"]
mod http_fixture;
#[path = "tests/http_recovery.rs"]
mod http_recovery;
#[path = "tests/outcomes.rs"]
mod outcomes;
#[path = "tests/owner_recovery.rs"]
mod owner_recovery;
#[path = "tests/refusal_recovery.rs"]
mod refusal_recovery;

use assertions::*;
use http_fixture::*;

use super::{Run, Tally, JOURNAL};
use crate::arbiter::collect::Options;
use crate::arbiter::parse::{OrgSchool, PrimaryContact};
use crate::net::{FetchOptions, Fetcher};
use crate::{AdapterContext, Recording};
use census_domain::model::{CanonicalCoach, CanonicalSchool, SchoolYear, SourceObservation};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const BASE: &str = "https://services.arbitersports.com/api/v2/organization/public/2132/children";
const AT: &str = "2026-10-01";

enum FirstPage {
    Missing,
    Malformed,
    Short,
    EmptyShort,
    LaterMalformed,
    Bounded,
    SamePageMalformed,
    MalformedField,
}

fn coach_url(page: u64) -> String {
    format!(
        "https://services.arbitersports.com/api/v2/legacy/public/2132/coaches?filter.EntityId=450&&pageSize=200&pageNumber={page}"
    )
}

fn coach(first_name: &str, last_name: &str) -> Value {
    json!({
        "firstName": first_name, "lastName": last_name,
        "coachPositionName": "Head Coach", "sportName": "Cross Country, Girls",
        "levelName": "Varsity"
    })
}

fn school() -> OrgSchool {
    OrgSchool {
        name: "Recovery High School".to_string(),
        public_id: Some(450),
        org_id: Some(29072),
        phone: None,
        enrollment: Some(955),
        primary_contact: Some(PrimaryContact {
            first_name: "Casey".to_string(),
            last_name: "Reed".to_string(),
            role_name: "Athletic Director".to_string(),
        }),
    }
}

fn seed(cache: &Path, page: u64, body: &str) -> TestResult {
    let url = coach_url(page);
    let key = Fetcher::key_for("GET", &url, "");
    std::fs::create_dir_all(cache)?;
    std::fs::write(cache.join(format!("{key}.body")), body)?;
    std::fs::write(
        cache.join(format!("{key}.meta.json")),
        json!({
            "url": url, "method": "GET", "status": 200,
            "content_digest": crate::net::cache::content_digest(body.as_bytes()),
            "bytes": body.len(), "fetched_at": "2026-10-01T12:00:00Z"
        })
        .to_string(),
    )?;
    Ok(())
}

fn seed_failure(cache: &Path, first: &FirstPage) -> TestResult<BTreeSet<String>> {
    let mut expected = BTreeSet::from(["Casey Reed".to_string()]);
    match first {
        FirstPage::Missing => {}
        FirstPage::Malformed => seed(cache, 1, "not JSON")?,
        FirstPage::EmptyShort => seed(
            cache,
            1,
            &json!({"data": {"total": 1, "rows": []}}).to_string(),
        )?,
        FirstPage::Short => {
            seed(
                cache,
                1,
                &json!({"data": {"total": 2, "rows": [coach("Ada", "Lane")]}}).to_string(),
            )?;
            expected.insert("Ada Lane".to_string());
        }
        FirstPage::SamePageMalformed | FirstPage::MalformedField => {
            let invalid = match first {
                FirstPage::MalformedField => json!({"firstName": 42}),
                _ => json!(42),
            };
            seed(
                cache,
                1,
                &json!({"data": {"total": 3, "rows": [
                    coach("Ada", "Lane"), invalid, coach("Beau", "Pine")
                ]}})
                .to_string(),
            )?;
            expected.extend(["Ada Lane".to_string(), "Beau Pine".to_string()]);
        }
        FirstPage::LaterMalformed | FirstPage::Bounded => {
            let rows: Vec<_> = std::iter::once(coach("Ada", "Lane"))
                .chain(std::iter::repeat_n(json!({}), 199))
                .collect();
            let body = json!({"data": {"total": 12801, "rows": rows}}).to_string();
            seed(cache, 1, &body)?;
            if matches!(first, FirstPage::Bounded) {
                (2..=64).try_for_each(|page| seed(cache, page, &body))?;
            } else {
                seed(cache, 2, "not JSON")?;
            }
            expected.insert("Ada Lane".to_string());
        }
    }
    Ok(expected)
}

fn context<'a>(
    fetcher: &'a Fetcher,
    store: &'a Store,
    recording: Option<&'a Recording>,
) -> TestResult<AdapterContext<'a>> {
    Ok(AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("school year")?,
        observed_on: AT.to_string(),
        recording,
    })
}

fn options() -> Options {
    Options {
        states: vec![UsJurisdiction::NewHampshire],
        observed_on: AT.to_string(),
        limit: None,
        refresh: false,
    }
}

fn run<'a>(ctx: &'a AdapterContext<'a>, options: &'a Options) -> TestResult<Run<'a>> {
    Ok(Run {
        ctx,
        options,
        fetch: FetchOptions::default(),
        done: ctx.store.journal_keys(JOURNAL)?,
        tally: Tally::default(),
    })
}

fn fetcher(cache: &Path) -> TestResult<Fetcher> {
    Ok(Fetcher::new(
        cache,
        None,
        Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )?
    .with_offline(true))
}

fn pending(store: &Store, row: &OrgSchool) -> TestResult<Value> {
    let owner = super::completion_key(UsJurisdiction::NewHampshire, "2132", row.public_id)
        .ok_or("public owner")?;
    Ok(store
        .journal_payloads(&super::recovery::phase(&owner))?
        .into_iter()
        .next()
        .ok_or("owed marker")?)
}

fn apply_recorded(store: &Store, recorded: &crate::recording::Recorded) -> TestResult {
    let mut batch = store.write_batch();
    for rows in &recorded.rows {
        batch.append_many(rows.table, &rows.rows)?;
    }
    for entry in &recorded.journal {
        batch.journal_done(&entry.phase, &entry.key, &entry.payload)?;
    }
    batch.commit()?;
    Ok(())
}
