use crate::model::SearchHit;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SearchCacheStatus {
    Success,
    Failure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchCacheRecord {
    pub key: String,
    pub query: String,
    pub filter: String,
    pub attempts: u32,
    pub status: SearchCacheStatus,
    pub hits: Vec<SearchHit>,
    pub error: String,
    pub retryable: bool,
    pub completed_at_unix: u64,
}

impl SearchCacheRecord {
    pub fn success(
        key: String,
        query: String,
        filter: String,
        attempts: u32,
        hits: Vec<SearchHit>,
    ) -> Self {
        Self {
            key,
            query,
            filter,
            attempts,
            status: SearchCacheStatus::Success,
            hits,
            error: String::new(),
            retryable: false,
            completed_at_unix: unix_time(),
        }
    }

    pub fn retryable(
        key: String,
        query: String,
        filter: String,
        attempts: u32,
        error: String,
    ) -> Self {
        Self {
            key,
            query,
            filter,
            attempts,
            status: SearchCacheStatus::Failure,
            hits: Vec::new(),
            error,
            retryable: true,
            completed_at_unix: unix_time(),
        }
    }

    pub fn is_success(&self) -> bool {
        self.status == SearchCacheStatus::Success
    }
}

pub fn load_latest(path: &Path) -> Result<HashMap<String, SearchCacheRecord>> {
    crate::jsonl::load(path, |record: &SearchCacheRecord| record.key.clone())
}

pub fn append(path: &Path, record: &SearchCacheRecord) -> Result<()> {
    crate::jsonl::append(path, record)
}

fn unix_time() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SearchHit;
    use anyhow::Context;
    use tempfile::tempdir;

    #[test]
    fn latest_success_replaces_retryable_failure() -> Result<()> {
        let directory = tempdir()?;
        let path = directory.path().join("search-cache.jsonl");
        append(
            &path,
            &SearchCacheRecord::retryable(
                "a:tf\nada runner".to_owned(),
                "Ada Runner".to_owned(),
                "a:tf".to_owned(),
                1,
                "timeout".to_owned(),
            ),
        )?;
        append(
            &path,
            &SearchCacheRecord::success(
                "a:tf\nada runner".to_owned(),
                "Ada Runner".to_owned(),
                "a:tf".to_owned(),
                2,
                vec![SearchHit {
                    url: "https://www.athletic.net/athlete/7/track-and-field".to_owned(),
                    ..Default::default()
                }],
            ),
        )?;

        let records = load_latest(&path)?;
        let record = records
            .get("a:tf\nada runner")
            .context("missing cached record")?;
        assert!(record.is_success());
        assert!(!record.retryable);
        assert_eq!(record.attempts, 2);
        assert_eq!(record.hits.len(), 1);
        Ok(())
    }

    #[test]
    fn malformed_cache_reports_the_exact_line() -> Result<()> {
        let directory = tempdir()?;
        let path = directory.path().join("search-cache.jsonl");
        let valid = serde_json::to_string(&SearchCacheRecord::success(
            "a:tf\nada".to_owned(),
            "Ada".to_owned(),
            "a:tf".to_owned(),
            1,
            Vec::new(),
        ))?;
        std::fs::write(&path, format!("{valid}\nnot-json\n"))?;
        let error = match load_latest(&path) {
            Ok(_) => anyhow::bail!("malformed cache unexpectedly loaded"),
            Err(error) => error,
        };
        assert!(error.to_string().contains("line 2"));
        Ok(())
    }
}
