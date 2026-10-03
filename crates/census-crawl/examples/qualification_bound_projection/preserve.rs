use super::input::Inputs;
use super::{io, Result};
use census_crawl::net::Fetcher;
use census_store::Store;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

pub(super) fn initialize(root: &Path, inputs: &Inputs) -> Result<()> {
    io::directory(root)?;
    io::directory(&root.join("inputs"))?;
    inputs
        .captures
        .iter()
        .try_for_each(|capture| -> Result<()> {
            io::write(
                &root.join("inputs").join(format!("{}.body", capture.key)),
                &capture.body,
            )?;
            io::write(
                &root
                    .join("inputs")
                    .join(format!("{}.meta.json", capture.key)),
                &capture.metadata_bytes,
            )
        })?;
    io::json(&root.join("input_contract.json"), &inputs.facts)?;
    io::json(&root.join("parsed_owned_capture.json"), &inputs.owned)
}

pub(super) fn seed(store: &Store, inputs: &Inputs) -> Result<()> {
    inputs
        .captures
        .iter()
        .try_for_each(|capture| -> Result<()> {
            io::write(
                &store.http_cache_dir().join(format!("{}.body", capture.key)),
                &capture.body,
            )?;
            io::write(
                &store
                    .http_cache_dir()
                    .join(format!("{}.meta.json", capture.key)),
                &capture.metadata_bytes,
            )
        })?;
    census_store::read::write_snapshot_rows::<census_domain::model::CanonicalSchool>(
        &store.out_dir().join("schools.jsonl"),
        &[],
    )?;
    Ok(())
}

pub(super) fn fetcher(store: &Store) -> Result<Fetcher> {
    Ok(Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["milesplit.com".into()],
    )?
    .with_offline(true))
}

pub(super) fn cache_originals(store: &Store, inputs: &Inputs) -> Result<Value> {
    let captures = inputs
        .captures
        .iter()
        .map(|capture| -> Result<Value> {
            let body = io::read(
                &store.http_cache_dir().join(format!("{}.body", capture.key)),
                io::MAX_BODY,
            )?;
            let meta = io::read(
                &store
                    .http_cache_dir()
                    .join(format!("{}.meta.json", capture.key)),
                io::MAX_META,
            )?;
            Ok(
                json!({"key": capture.key, "body_identical": body == capture.body,
            "metadata_identical": meta == capture.metadata_bytes, "sha256": io::digest(&body)}),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    let unchanged = captures.iter().all(|capture| {
        capture.get("body_identical").and_then(Value::as_bool) == Some(true)
            && capture.get("metadata_identical").and_then(Value::as_bool) == Some(true)
    });
    Ok(json!({"unchanged": unchanged, "captures": captures}))
}
