use super::*;
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use census_domain::model::{CanonicalEvent, CanonicalPerformance, EventKind, SchoolYear};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::{BTreeMap, BTreeSet, HashMap};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const URL: &str = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
const HISTORICAL_PHASE: &str = "tfrrs_projection_v3";
const LIST: &str = include_str!("../../tests/fixtures/tfrrs/indiana_list_5489_excerpt.html");

#[path = "run_receipts_date_tests.rs"]
mod dates;

fn context<'a>(
    store: &'a Store,
    fetcher: &'a crate::net::Fetcher,
    capture: &'a crate::ResultsCaptureContext,
) -> TestResult<AdapterContext<'a>> {
    Ok(AdapterContext {
        capture: crate::CaptureContext::DurableV1(capture),
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("school year")?,
        observed_on: "2099-01-01".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 7).ok_or("snapshot date")?,
        recording: None,
    })
}

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

fn seed(fetcher: &crate::net::Fetcher, body: &[u8]) -> TestResult {
    let (body_path, meta_path) = fetcher.cache_paths(&crate::net::Fetcher::key_for("GET", URL, ""));
    std::fs::create_dir_all(fetcher.cache_dir())?;
    write_cache(
        &body_path,
        &meta_path,
        body,
        &CacheMeta {
            url: URL.into(),
            method: "GET".into(),
            status: 200,
            content_digest: content_digest(body),
            bytes: body.len(),
            fetched_at: "2026-09-22T03:55:31Z".into(),
            ..CacheMeta::default()
        },
    )?;
    Ok(())
}

fn schools(store: &Store, body: &str) -> TestResult<Vec<CanonicalSchool>> {
    let labels: BTreeSet<_> = crate::tfrrs::parse_list_page(body)
        .sections
        .into_iter()
        .flat_map(|section| section.rows)
        .filter_map(|row| row.team.map(|team| team.name))
        .collect();
    for name in labels {
        let school = CanonicalSchool::new(
            UsJurisdiction::Indiana,
            &name,
            census_domain::model::normalize_name(&name),
            None,
        )
        .0;
        store.append(Table::Schools, &school)?;
    }
    Ok(store.scan(Table::Schools)?)
}

async fn collect(ctx: &AdapterContext<'_>) -> TestResult<crate::AdapterReport> {
    Ok(crate::tfrrs::collect(
        ctx,
        &crate::tfrrs::Options {
            urls: vec![URL.into()],
            ..crate::tfrrs::Options::default()
        },
    )
    .await?)
}

#[test]
fn v3_completion_does_not_suppress_current_xc_interpretation_and_unchanged_row_effects_replay_once(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let dir = tempfile::tempdir()?;
        let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
        let store = Store::open(dir.path().join("store"))?;
        let fetcher = crate::net::Fetcher::new(dir.path().join("http"), None,
            std::time::Duration::ZERO, HashMap::new(), Vec::new())?.with_offline(true);
        let body = LIST.replace("60 Meters", "XC 5K");
        seed(&fetcher, body.as_bytes())?;
        let bindings = schools(&store, &body)?;
        let ctx = context(&store, &fetcher, &capture)?;
        let old_context = digest(&(HISTORICAL_PHASE, ctx.performance_as_of,
            ctx.school_year, &bindings))?;
        let old_key = key(&old_context, URL, &content_digest(body.as_bytes()))?;
        let historical = serde_json::json!({"url": URL, "kind": "list", "rows": 28,
            "body_sha256": content_digest(body.as_bytes()), "projection_context": old_context});
        store.journal_done(HISTORICAL_PHASE, &old_key, &historical)?;
        let history = snapshot(&store, HISTORICAL_PHASE)?;
        check!(eq; PHASE, "tfrrs_projection_v5");
        let new_context = super::context(&ctx, &bindings)?;
        check!(ne; new_context, old_context);
        let new_key = key(&new_context, URL, &content_digest(body.as_bytes()))?;
        check!(ne; new_key, old_key);
        let applied = collect(&ctx).await?;
        check!(eq; applied.errors, 0, "{:?}", applied.notes);
        check!(eq; applied.requests, 0);
        let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
        let control = performances.iter().find(|row| row.source_athlete.as_ref()
            .is_some_and(|owner| owner.id == "8817192")).ok_or("published control performance")?;
        check!(eq; control.mark, census_domain::model::Mark::TimeSeconds(
            census_domain::model::ExactSeconds::parse("6.97")?));
        let events: Vec<CanonicalEvent> = store.scan(Table::Events)?;
        check!(eq; events.iter().find(|event| event.id == control.event)
            .ok_or("XC event reference")?.kind, EventKind::CrossCountry);
        let current = snapshot(&store, PHASE)?;
        check!(eq; current.len(), 1);
        check!(eq; current.get(&new_key).ok_or("new completion identity")?["projection_context"],
            new_context);
        check!(eq; snapshot(&store, HISTORICAL_PHASE)?, history);
        let physical = store.walk_table(Table::Performances)?;
        drop(store);
        let store = Store::open(dir.path().join("store"))?;
        let repeated = collect(&context(&store, &fetcher, &capture)?).await?;
        check!(eq; repeated.errors, 0);
        check!(eq; repeated.requests, 0);
        check!(eq; store.walk_table(Table::Performances)?, physical);
        check!(eq; snapshot(&store, PHASE)?, current);
        check!(eq; snapshot(&store, HISTORICAL_PHASE)?, history);
        check!(eq; fetcher.stats().await.physical_requests(), 0);
        Ok(())
    })
}
