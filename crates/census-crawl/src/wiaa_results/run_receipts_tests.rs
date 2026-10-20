use super::*;
use census_domain::model::SchoolYear;
use census_store::Store;
use std::collections::HashMap;
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn changed_event_grammar_has_fresh_context_and_preserves_prior_completion() -> TestResult {
    let directory = tempfile::tempdir()?;
    let capture = crate::net::durable_capture_context(&directory.path().join("capture"))?;
    let store = Store::open(directory.path())?;
    let fetcher = crate::net::Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )?
    .with_source("wiaa_results")
    .with_offline(true);
    let ctx = AdapterContext {
        capture: crate::CaptureContext::DurableV1(&capture),
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("school year")?,
        observed_on: "2026-10-13".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 13).ok_or("as of")?,
        recording: None,
    };
    let schools: Vec<CanonicalSchool> = Vec::new();
    let prior_context = digest(&(
        "wiaa_results_projection_v2",
        8_u32,
        ctx.performance_as_of,
        ctx.school_year,
        &schools,
    ))?;
    let current_context = context(&ctx, &schools)?;
    check!(eq; PHASE, "wiaa_results_projection_v3");
    check!(eq; super::super::PARSE_VERSION, 9);
    check!(ne; current_context, prior_context);
    check!(eq; current_context, context(&ctx, &schools)?);
    let url = "https://example.test/wiaa/retained-results";
    let body = b"Boys XC 3 Miles";
    let prior_key = key(&prior_context, url, body)?;
    let current_key = key(&current_context, url, body)?;
    check!(ne; prior_key, current_key);
    let prior = serde_json::json!({"parser": 8, "retained": true});
    store.journal_done("wiaa_results_projection_v2", &prior_key, &prior)?;
    check!(store.journal_keys(PHASE)?.is_empty());
    let current = serde_json::json!({"parser": 9, "retained": true});
    store.journal_done(PHASE, &current_key, &current)?;
    check!(eq; store.journal_payload("wiaa_results_projection_v2", &prior_key)?, Some(prior.clone()));
    check!(eq; store.journal_payload(PHASE, &current_key)?, Some(current.clone()));
    drop(ctx);
    drop(store);
    let reopened = Store::open(directory.path())?;
    check!(eq; reopened.journal_payload("wiaa_results_projection_v2", &prior_key)?, Some(prior));
    check!(eq; reopened.journal_payload(PHASE, &current_key)?, Some(current));
    Ok(())
}
