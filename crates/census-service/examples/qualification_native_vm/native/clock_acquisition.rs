use super::super::{artifacts, ENDPOINT, GUEST};
use anyhow::{bail, ensure, Context, Result};
use census_crawl::net::{FetchOptions, FetchOutcome, FetchStats, Fetcher};
use census_crawl::{registry, riil};
use census_store::Store;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;

mod checks;
mod persistence;
#[cfg(test)]
mod tests;

const URL: &str = "https://riil.org/Directory.aspx";
const JOURNAL: &str = "native_clock_acquisitions_v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
enum Phase {
    Before,
    After,
}

impl Phase {
    fn parse(value: &str) -> Result<Self> {
        match value {
            "before" => Ok(Self::Before),
            "after" => Ok(Self::After),
            other => bail!("unsupported acquisition phase: {other}"),
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::After => "after",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Identity {
    run: String,
    season: u16,
    cohort: u16,
    revision: u32,
    source: String,
    source_unit: String,
    manifest_sha256: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct CapturePaths {
    body: PathBuf,
    metadata: PathBuf,
    metadata_sha256: String,
    archive_body: PathBuf,
    archive_metadata: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct JournalRef {
    store_root: PathBuf,
    phase: String,
    key: String,
    readback: PathBuf,
}

#[derive(Debug, Serialize, Deserialize)]
struct Record {
    phase: Phase,
    identity: Identity,
    capture: FetchOutcome,
    paths: CapturePaths,
    journal: JournalRef,
    clock_before: Value,
    clock: Value,
    stats: FetchStats,
    schools: usize,
    xc_tf_appointments: usize,
}

struct Session {
    store: Store,
    fetcher: Fetcher,
    identity: Identity,
    previous: Option<Record>,
    root: PathBuf,
}

#[tracing::instrument]
pub(super) async fn acquire(phase: &str) -> Result<Value> {
    let phase = Phase::parse(phase)?;
    let session = tokio::task::spawn_blocking(move || prepare(phase))
        .await
        .context("acquisition preparation task failed")??;
    let clock_before = super::clock::clock()?;
    let acquired = fetch(&session.fetcher).await;
    tokio::task::spawn_blocking(move || {
        let result = persistence::finish(&session, phase, clock_before, acquired);
        let persisted = session.store.flush();
        drop(session);
        match (result, persisted) {
            (Ok(value), Ok(())) => Ok(json!({
                "acquisition": value,
                "store_flushed": true,
                "store_closed": true,
                "transport": "production census_crawl::net::Fetcher public HTTPS",
                "scope": "isolated guest acquisition clock proof only",
                "national_census_claim": false,
                "public_freshness_claim": false,
                "retry_owner": "none: one qualification attempt per phase"
            })),
            (Err(error), Ok(())) => Err(error),
            (Ok(_), Err(error)) => Err(error.into()),
            (Err(error), Err(flush)) => {
                Err(error.context(format!("acquisition store flush also failed: {flush}")))
            }
        }
    })
    .await
    .context("acquisition finalization task failed")?
}

#[tracing::instrument(skip(fetcher))]
async fn fetch(fetcher: &Fetcher) -> Result<(FetchOutcome, FetchStats)> {
    let outcome = fetcher
        .get(
            URL,
            &FetchOptions {
                refresh: true,
                ..FetchOptions::default()
            },
        )
        .await
        .context("fresh physical RIIL directory acquisition failed")?;
    Ok((outcome, fetcher.stats().await))
}

fn prepare(phase: Phase) -> Result<Session> {
    let guest = std::fs::canonicalize(GUEST)?;
    let root = guest.join("clock-acquisition-store");
    if phase == Phase::Before && !root.try_exists()? {
        std::fs::create_dir(&root)?;
    }
    ensure!(
        std::fs::canonicalize(&root)? == root,
        "acquisition root must not be a symlink"
    );
    persistence::ensure_owned_directories(&root)?;
    let identity = identity()?;
    let store = Store::open(&root).context("opening distinct guest-owned acquisition store")?;
    let previous = persistence::previous(&store, &root, &identity, phase)?;
    let descriptor = registry::descriptor("riil").context("RIIL source descriptor absent")?;
    ensure!(
        descriptor.transport == registry::TransportKind::Html
            && descriptor.access_class() == registry::AccessClass::Open
            && descriptor.admission.origin == "riil.org",
        "RIIL source policy is not public HTML at the qualified origin"
    );
    let delay = registry::declared_delay_for_host(descriptor.admission.origin)
        .context("RIIL declared origin budget absent")?;
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        delay,
        HashMap::new(),
        vec![descriptor.admission.origin.to_owned()],
    )?
    .with_source(riil::SOURCE_ID);
    Ok(Session {
        store,
        fetcher,
        identity,
        previous,
        root,
    })
}

fn identity() -> Result<Identity> {
    let bytes = artifacts::read(&PathBuf::from(GUEST).join("manifest.json"))?;
    let manifest: Value = serde_json::from_slice(&bytes)?;
    let run = manifest
        .get("run")
        .and_then(Value::as_str)
        .context("manifest run absent")?;
    ensure!(
        run == ENDPOINT,
        "acquisition run differs from owning harness endpoint"
    );
    let number = |name: &str| -> Result<u64> {
        manifest
            .get(name)
            .and_then(Value::as_u64)
            .with_context(|| format!("manifest {name} absent"))
    };
    let season = u16::try_from(number("season")?)?;
    let cohort = u16::try_from(number("cohort")?)?;
    let revision = u32::try_from(number("revision")?)?;
    ensure!(
        season == 2026 && cohort == 2027 && revision == 1,
        "unexpected harness season/cohort/revision"
    );
    Ok(Identity {
        run: run.to_owned(),
        season,
        cohort,
        revision,
        source: riil::SOURCE_ID.to_owned(),
        source_unit: format!("{run}:RI:{}:{URL}", riil::SOURCE_ID),
        manifest_sha256: artifacts::sha(&bytes),
    })
}
