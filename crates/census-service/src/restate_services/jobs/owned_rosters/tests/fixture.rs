use super::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) const HOST: &str = "mi.milesplit.com";

pub(super) fn fetcher(root: &Path) -> TestResult<Fetcher> {
    Ok(Fetcher::new(
        root.join("controlled-http"),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec![],
    )?
    .with_offline(true)
    .with_origin_locks(root.join("controlled-locks")))
}

pub(super) fn failure(status: u16) -> Result<CapturedPublicDirectory, FetchError> {
    Err(FetchError::Http {
        status,
        url: "https://mi.milesplit.com/teams".into(),
    })
}

#[derive(Deserialize)]
pub(super) struct Index {
    pub(super) disposition: CollectionDisposition,
    pub(super) errors: usize,
}

pub(super) fn index(store: &Store) -> TestResult<Index> {
    let value = store
        .journal_payload(&census::teams_phase(UsJurisdiction::Michigan), "MI")?
        .ok_or("controlled index receipt missing")?;
    Ok(serde_json::from_value(value)?)
}
