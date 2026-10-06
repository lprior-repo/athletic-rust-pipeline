use super::*;
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use census_domain::model::{CanonicalSchool, SourceIdentity, SourceNamespace};
use census_domain::UsJurisdiction;

pub(super) const TROY: &[u8] = include_bytes!("../../../owned/fixtures/troy_725218.json");
pub(super) const RELAYS: &[u8] =
    include_bytes!("../../../../../tests/fixtures/milesplit/troy_725218_team_relays.json");
pub(super) const FEMALE_RAW: &[u8] = include_bytes!(
    "../../../../../tests/fixtures/milesplit/troy_725218_rs1266814_raw_projection.html"
);
pub(super) const MALE_RAW: &[u8] = include_bytes!(
    "../../../../../tests/fixtures/milesplit/troy_725218_rs1266815_raw_projection.html"
);

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
) -> TestResult<AdapterContext<'a>> {
    Ok(AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("school year")?,
        observed_on: "2099-01-01".into(),
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

pub(super) fn seed_metadata(fetcher: &Fetcher, reference: &ResultSetRef) -> TestResult {
    let raw = match reference.rsid.as_str() {
        "1266814" => FEMALE_RAW,
        "1266815" => MALE_RAW,
        id => return Err(format!("no authentic Troy metadata fixture for result set {id}").into()),
    };
    seed(fetcher, &reference.url, raw)
}

pub(super) fn options(reference: &ResultSetRef) -> super::super::super::ResultSetOptions {
    super::super::super::ResultSetOptions {
        urls: vec![super::super::super::ResultSetRequest {
            url: reference.url.clone(),
            jurisdiction: UsJurisdiction::Alabama,
        }],
    }
}

pub(super) fn apply(store: &Store, recording: &crate::recording::Recorded) -> TestResult {
    let mut batch = store.write_batch();
    for rows in &recording.rows {
        batch.append_many(rows.table, &rows.rows)?;
    }
    for entry in &recording.journal {
        batch.journal_done(&entry.phase, &entry.key, &entry.payload)?;
    }
    batch.commit()?;
    Ok(())
}
