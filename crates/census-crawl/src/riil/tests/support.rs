use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::{FetchOutcome, Fetcher};
use crate::{AdapterContext, AdapterReport, Recording};
use census_domain::model::SchoolYear;
use census_store::{Entity, Store, Table};
use std::{collections::HashMap, time::Duration};

pub(super) const URL: &str = "https://riil.org/Directory.aspx";
pub(super) const ACQUIRED_AT: &str = "2026-08-31T09:00:00Z";
pub(super) const DIRECTORY: &str = r#"<div id="BodyWrapper"><ul><li><details>
<summary>Example HS</summary><table class='DirectoryStaffTable'>
<tr><th></th><th>Role</th><th>Name</th><th>Phone</th></tr>
<tr><td>Boys Cross Country</td><td>Head Coach</td><td>Alex Coach</td><td><a href="tel:401-555-0100">Call</a></td></tr>
<tr><td>Girls Outdoor Track</td><td>Head Coach</td><td>Morgan Coach</td><td></td></tr>
<tr><td>Boys Basketball</td><td>Head Coach</td><td>Other Coach</td><td></td></tr>
</table></details></li></ul></div>"#;

pub(super) fn capture(body: &str) -> FetchOutcome {
    FetchOutcome {
        url: URL.to_string(),
        response_url: None,
        method: "GET".to_string(),
        status: 200,
        content_digest: content_digest(body.as_bytes()),
        bytes: body.len(),
        fetched_at: ACQUIRED_AT.to_string(),
        from_cache: true,
        content_type: Some("text/html".to_string()),
        body: body.as_bytes().to_vec(),
    }
}

pub(super) fn setup() -> anyhow::Result<(tempfile::TempDir, Store, Fetcher)> {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path().join("store"))?;
    let fetcher = fetcher(&store)?.with_offline(true);
    Ok((root, store, fetcher))
}

pub(super) fn fetcher(store: &Store) -> anyhow::Result<Fetcher> {
    Ok(Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["127.0.0.1".to_string()],
    )?)
}

pub(super) fn seed(fetcher: &Fetcher, body: &[u8], fetched_at: &str) -> anyhow::Result<()> {
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", URL, ""));
    let meta = CacheMeta {
        representation: crate::net::RepresentationHeaders::default(),
        url: URL.to_string(),
        method: "GET".to_string(),
        status: 200,
        content_digest: content_digest(body),
        bytes: body.len(),
        fetched_at: fetched_at.to_string(),
        content_type: Some("text/html".to_string()),
        ..CacheMeta::default()
    };
    write_cache(&body_path, &meta_path, body, &meta)?;
    Ok(())
}

pub(super) async fn evaluate(
    fetcher: &Fetcher,
    store: &Store,
    recording: Option<&Recording>,
    evaluation: &str,
    option_date: &str,
) -> anyhow::Result<AdapterReport> {
    let ctx = context(fetcher, store, recording, evaluation);
    Ok(super::super::collect(
        &ctx,
        &super::super::Options {
            observed_on: option_date.to_string(),
            ..Default::default()
        },
    )
    .await?)
}

pub(super) fn context<'a>(
    fetcher: &'a Fetcher,
    store: &'a Store,
    recording: Option<&'a Recording>,
    evaluation: &str,
) -> AdapterContext<'a> {
    AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::DEFAULT,
        observed_on: evaluation.to_string(),
        recording,
    }
}

pub(super) fn observations<T: Entity>(store: &Store, table: Table) -> anyhow::Result<Vec<T>> {
    let mut rows = Vec::new();
    store.snapshot().for_each_observation(table, |row| {
        rows.push(row);
        Ok(())
    })?;
    Ok(rows)
}

pub(super) fn physical_counts(store: &Store) -> anyhow::Result<Vec<u64>> {
    [Table::Schools, Table::Coaches, Table::SourceObservations]
        .into_iter()
        .map(|table| Ok(store.walk_table(table)?.rows))
        .collect()
}
