use census_crawl::net::FetchOutcome;
use census_domain::model::{CanonicalAthlete, CanonicalPerformance, CanonicalSchool};
use census_domain::model::{SourceIdentity, SourceNamespace};
use census_store::{Store, Table};
use serde_json::json;
use sha2::{Digest, Sha256};

mod evidence;
mod results;
mod roster;

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

pub(super) const PROVIDER_TEAM_ID: &str = "95010";

pub(super) fn qualify(
    store: &Store,
    school: &CanonicalSchool,
    athlete: &CanonicalAthlete,
) -> TestResult<CanonicalPerformance> {
    let mut provider_school = school.clone();
    if provider_school
        .source_identities
        .iter()
        .all(|source| source.namespace != SourceNamespace::MilesplitSchool)
    {
        provider_school.source_identities.push(SourceIdentity::new(
            SourceNamespace::MilesplitSchool,
            PROVIDER_TEAM_ID,
        ));
    }
    store.append(Table::Schools, &provider_school)?;
    let mut performance = block_on(results::append(store, &provider_school, athlete))?;
    check!(eq; performance.athlete, athlete.id);
    roster::append(store, &athlete.id)?;
    evidence::append(store, &mut performance)?;
    Ok(performance)
}

pub(super) fn athlete_profile_url(host: &str, athlete_id: &str, canonical_name: &str) -> String {
    format!(
        "https://{}/athletes/{}-{}",
        host,
        athlete_id,
        canonical_name.to_lowercase().replace(' ', "-")
    )
}

pub(super) fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn retain(store: &Store, capture: &FetchOutcome) -> TestResult<()> {
    let root = store.http_cache_dir().join("archive");
    let captures = root.join("captures").join(&capture.content_digest);
    std::fs::create_dir_all(root.join("bodies"))?;
    std::fs::create_dir_all(&captures)?;
    std::fs::write(
        root.join("bodies")
            .join(format!("{}.body", capture.content_digest)),
        &capture.body,
    )?;
    let metadata = serde_json::to_vec(&json!({
        "url": capture.url, "response_url": null, "method": "GET", "status": 200,
        "content_digest": capture.content_digest, "bytes": capture.bytes,
        "fetched_at": capture.fetched_at, "content_type": "text/html", "etag": "synthetic fixture", "redirects": [],
    }))?;
    std::fs::write(
        captures.join(format!("{}.meta.json", hash(&metadata))),
        metadata,
    )?;
    Ok(())
}

fn block_on<T>(ingest: impl std::future::Future<Output = TestResult<T>>) -> TestResult<T> {
    if tokio::runtime::Handle::try_current().is_ok() {
        return Err("selected fixture ingestion requires no ambient async runtime".into());
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    Ok(runtime.block_on(ingest)?)
}
