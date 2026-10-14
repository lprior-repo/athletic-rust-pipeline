use super::*;
use crate::net::Fetcher;
use census_domain::model::SchoolYear;
use census_store::Store;
use std::{collections::HashMap, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn ca_recorded_details_project_a_school_without_source_errors() -> TestResult {
    project_capture(
        include_str!("../../../../tests/fixtures/home_campus/details_19.json"),
        1,
        19,
    )
}

#[test]
fn nj_recorded_empty_staff_arrays_do_not_invalidate_school_inventory() -> TestResult {
    project_capture(
        include_str!("../../../../tests/fixtures/home_campus/details_3374.json"),
        12,
        3374,
    )
}

fn project_capture(body: &str, section_id: u32, school_id: u64) -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path().join("store"))?;
    let fetcher = Fetcher::new(
        root.path().join("http"),
        None,
        Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )?
    .with_offline(true);
    let context = AdapterContext {
        capture: crate::CaptureContext::LegacyMetadata,
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("2026 season")?,
        observed_on: "2026-10-09".into(),
        performance_as_of: chrono::NaiveDate::parse_from_str("2026-10-09", "%Y-%m-%d")?,
        recording: None,
    };
    let section = crate::home_campus::SECTIONS
        .iter()
        .find(|section| section.number == section_id)
        .ok_or("registered source section")?;
    let parsed: Value = serde_json::from_str(body)?;
    let profile = crate::home_campus::parse::parse_school_details(body)
        .ok_or("recorded school details")?
        .profile;
    let url = format!("https://www.cifsshome.org/widget/get-school-details/{school_id}/details");
    let mut report = AdapterReport::new("home_campus", "schools");
    let capture = crate::net::FetchOutcome {
        url,
        response_url: None,
        method: "GET".to_owned(),
        status: 200,
        content_digest: crate::net::cache::content_digest(body.as_bytes()),
        bytes: body.len(),
        fetched_at: "2026-10-04T12:00:00Z".to_owned(),
        from_cache: true,
        content_type: Some("application/json".to_owned()),
        body: body.as_bytes().to_vec(),
    };
    project(
        &context,
        section,
        &capture,
        (&profile, &parsed),
        &mut report,
    )?;
    check!(eq; report.rows, 1);
    check!(eq; report.errors, 0, "{:?}", report.notes);
    check!(report.unfinished.is_empty(), "{:?}", report.unfinished);
    Ok(())
}
