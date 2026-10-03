use super::{fixture_directory, fixture_staff_bonny_eagle};
use crate::mpa::{collect, Options, HOST_WWW};
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::Fetcher;
use crate::AdapterContext;
use census_domain::model::{
    CanonicalCoach, CanonicalSchool, Evidence, SchoolYear, SourceObservation,
};
use census_domain::UsJurisdiction;
use census_store::{Entity, Store, Table};
use std::{collections::HashMap, time::Duration};

const ACQUIRED_AT: &str = "2026-09-01T10:00:00Z";
const DIRECTORY_AT: &str = "2026-08-31T09:00:00Z";
const DIRECTORY_URL: &str = "https://www.mpa.cc/SchoolPages/School.aspx";
const STAFF_URL: &str = "https://www.mpa.cc/SchoolPages/School.aspx?SchoolID=3&tab=staff";

#[test]
fn collect_retains_cached_acquisition_when_execution_and_options_are_later(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let root = tempfile::tempdir()?;
            let store = Store::open(root.path())?;
            let fetcher = Fetcher::new(
                store.http_cache_dir(),
                None,
                Duration::ZERO,
                HashMap::new(),
                Vec::new(),
            )?
            .with_offline(true);
            seed(
                &fetcher,
                &format!("{HOST_WWW}/SchoolPages/School.aspx"),
                fixture_directory(),
                DIRECTORY_AT,
            )?;
            seed(
                &fetcher,
                STAFF_URL,
                fixture_staff_bonny_eagle(),
                ACQUIRED_AT,
            )?;
            evaluate(&fetcher, &store, "2026-10-02T12:00:00Z", "").await?;
            evaluate(
                &fetcher,
                &store,
                "2026-10-03T12:00:00Z",
                "2026-10-04T12:00:00Z",
            )
            .await?;
            let stats = fetcher.stats().await;
            check!(eq; stats.cache_hits, 4);
            check!(eq; stats.physical_requests(), 0);
            store.flush()?;
            drop(store);
            let reopened = Store::open(root.path())?;
            assert_readback(&reopened)?;
            Ok(())
        })
}

fn seed(fetcher: &Fetcher, url: &str, body: &str, fetched_at: &str) -> anyhow::Result<()> {
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", url, ""));
    let meta = CacheMeta {
        url: url.to_string(),
        method: "GET".to_string(),
        status: 200,
        content_digest: content_digest(body.as_bytes()),
        bytes: body.len(),
        fetched_at: fetched_at.to_string(),
        content_type: Some("text/html".to_string()),
        ..CacheMeta::default()
    };
    write_cache(&body_path, &meta_path, body.as_bytes(), &meta)?;
    Ok(())
}

async fn evaluate(
    fetcher: &Fetcher,
    store: &Store,
    evaluation: &str,
    option_date: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let ctx = AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::DEFAULT,
        observed_on: evaluation.to_string(),
        recording: None,
    };
    let options = Options {
        limit: Some(1),
        observed_on: option_date.to_string(),
        states: vec![UsJurisdiction::Maine],
        school_names: vec!["Bonny Eagle High School".to_string()],
        ..Options::default()
    };
    let report = collect(&ctx, &options).await?;
    check!(eq; report.rows, 1);
    check!(eq; report.errors, 0);
    check!(eq; report.from_cache, 2);
    Ok(())
}

fn observations<T: Entity>(store: &Store, table: Table) -> anyhow::Result<Vec<T>> {
    let mut rows = Vec::new();
    store.snapshot().for_each_observation(table, |row| {
        rows.push(row);
        Ok(())
    })?;
    Ok(rows)
}

fn assert_readback(store: &Store) -> Result<(), Box<dyn std::error::Error>> {
    let schools = observations::<CanonicalSchool>(store, Table::Schools)?;
    let coaches = observations::<CanonicalCoach>(store, Table::Coaches)?;
    let sources = observations::<SourceObservation>(store, Table::SourceObservations)?;
    check!(eq; schools.len(), 2);
    check!(eq; coaches.len(), 12);
    check!(eq; sources.len(), 2);
    schools.iter().try_for_each(|school| {
        check!(eq; school.name, "Bonny Eagle High School");
        check!(eq; school.evidence.len(), 1);
        assert_evidence(
            school.evidence.first().ok_or("school evidence")?,
            DIRECTORY_URL,
            DIRECTORY_AT,
            fixture_directory(),
        )
    })?;
    coaches.iter().try_for_each(|coach| {
        check!(eq; coach.professional_email, None);
        check!(eq; coach.evidence.len(), 1);
        assert_evidence(
            coach.evidence.first().ok_or("coach evidence")?,
            STAFF_URL,
            ACQUIRED_AT,
            fixture_staff_bonny_eagle(),
        )
    })?;
    sources.iter().try_for_each(|source| {
        let SourceObservation::School(school) = source else {
            return Err("unexpected athlete observation".into());
        };
        check!(eq; school.source_school_id, "mpa:3");
        check!(eq; school.observed_name, "Bonny Eagle High School");
        check!(eq; school.observed_on, DIRECTORY_AT);
        Ok(())
    })
}

fn assert_evidence(
    evidence: &Evidence,
    url: &str,
    acquired_at: &str,
    body: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    check!(eq; evidence.observed_on, acquired_at);
    check!(eq; evidence.source.id, "mpa_directory");
    check!(eq; evidence.source.url.as_deref(), Some(url));
    let note: serde_json::Value = serde_json::from_str(
        evidence
            .note
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("missing capture provenance"))?,
    )?;
    check!(eq; note["capture_url"], url);
    check!(eq; note["acquired_at"], acquired_at);
    check!(eq; note["sha256"], content_digest(body.as_bytes()));
    Ok(())
}
