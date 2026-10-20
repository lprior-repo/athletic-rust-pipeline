use super::*;
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use census_domain::model::{CanonicalSchool, SchoolYear, SourceIdentity, SourceNamespace};
use census_domain::UsJurisdiction;

pub(super) use crate::milesplit::troy_fixture::{FEMALE_RAW, MALE_RAW};

pub(super) fn setup_bare() -> TestResult<(tempfile::TempDir, Store, Fetcher, ResultSetRef)> {
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
            .ok_or("reference")?;
    Ok((dir, store, fetcher, reference))
}

pub(super) fn setup() -> TestResult<(tempfile::TempDir, Store, Fetcher, ResultSetRef)> {
    let (dir, store, fetcher, reference) = setup_bare()?;
    for school in [
        school("Spann provider school", "38332"),
        school("Charles", "4912"),
    ] {
        store.append(census_store::Table::Schools, &school)?;
    }
    Ok((dir, store, fetcher, reference))
}

pub(super) fn school(name: &str, team_id: &str) -> CanonicalSchool {
    let mut school = CanonicalSchool::new(
        UsJurisdiction::Alabama,
        name,
        census_domain::model::normalize_name(name),
        None,
    )
    .0;
    school.source_identities.push(SourceIdentity::new(
        SourceNamespace::MilesplitSchool,
        team_id,
    ));
    school
}

pub(super) fn context<'a>(
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
        observed_on: "2099-01-01".into(),
        performance_as_of: chrono::NaiveDate::parse_from_str("2026-10-07", "%Y-%m-%d")?,
        recording: None,
    })
}

pub(super) fn seed(fetcher: &Fetcher, url: &str, body: &[u8]) -> TestResult {
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", url, ""));
    std::fs::create_dir_all(fetcher.cache_dir())?;
    write_cache(
        &body_path,
        &meta_path,
        body,
        &CacheMeta {
            redirects: Vec::new(),
            representation: crate::net::RepresentationHeaders::default(),
            url: url.into(),
            method: "GET".into(),
            status: 200,
            content_digest: content_digest(body),
            bytes: body.len(),
            fetched_at: "2026-10-01T23:44:16Z".into(),
            ..CacheMeta::default()
        },
    )?;
    Ok(())
}

pub(super) fn seed_owned(fetcher: &Fetcher, reference: &ResultSetRef, body: &[u8]) -> TestResult {
    seed(
        fetcher,
        &crate::milesplit::fetch::owned_meet_url(reference)?,
        body,
    )
}
