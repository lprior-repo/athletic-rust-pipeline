use census_crawl::{net::Fetcher, AdapterContext};
use census_domain::model::SchoolYear;
use census_store::Store;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, time::Duration};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    pub(super) store: Store,
    pub(super) fetcher: Fetcher,
}

impl Fixture {
    pub(super) fn new() -> TestResult<Self> {
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
        Ok(Self {
            _root: root,
            store,
            fetcher,
        })
    }

    pub(super) fn context(&self) -> TestResult<AdapterContext<'_>> {
        Ok(AdapterContext {
            fetcher: &self.fetcher,
            store: &self.store,
            refresh: false,
            school_year: SchoolYear::new(2026).ok_or("2026 school year")?,
            observed_on: "2026-10-09".into(),
            performance_as_of: chrono::NaiveDate::parse_from_str("2026-10-09", "%Y-%m-%d")?,
            recording: None,
        })
    }

    pub(super) fn seed(&self, url: &str, body: &str) -> TestResult {
        let mut hasher = Sha256::new();
        hasher.update(b"GET");
        hasher.update([0x1f]);
        hasher.update(url.as_bytes());
        hasher.update([0x1f]);
        let key: String = hasher
            .finalize()
            .iter()
            .take(16)
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let cache = self.fetcher.cache_dir();
        std::fs::write(cache.join(format!("{key}.body")), body)?;
        std::fs::write(
            cache.join(format!("{key}.meta.json")),
            serde_json::to_vec(&serde_json::json!({
                "url": url, "method": "GET", "status": 200,
                "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
                "bytes": body.len(), "fetched_at": "2026-10-09T00:00:00Z",
            }))?,
        )?;
        Ok(())
    }
}
