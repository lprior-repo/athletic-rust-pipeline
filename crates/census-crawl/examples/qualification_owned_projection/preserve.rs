use super::input::{cache_key, Inputs};
use super::{io, Result};
use census_crawl::net::Fetcher;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

pub(super) fn initialize(root: &Path, inputs: &Inputs) -> Result<()> {
    io::directory(root)?;
    io::directory(&root.join("inputs"))?;
    let copies = [
        ("api.body", inputs.api.body.as_slice()),
        ("api.meta.json", inputs.api.metadata_bytes.as_slice()),
        ("female_raw.body", inputs.raw.body.as_slice()),
        ("female_raw.meta.json", inputs.raw.metadata_bytes.as_slice()),
        ("canonical_schools.jsonl", inputs.schools.as_slice()),
    ];
    copies
        .into_iter()
        .try_for_each(|(name, bytes)| io::write(&root.join("inputs").join(name), bytes))?;
    io::json(&root.join("input_contract.json"), &inputs.facts)?;
    save_sources(&root.join("sources"))?;
    Ok(())
}

pub(super) fn fetcher(store: &census_store::Store) -> Result<Fetcher> {
    Ok(Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["milesplit.com".into()],
    )?
    .with_offline(true))
}

pub(super) fn seed(store: &census_store::Store, inputs: &Inputs) -> Result<()> {
    let captures = [
        (&inputs.api, "52e0b5d61b6c7de90be2a35dd5fc3f42"),
        (&inputs.raw, "c7791d114036f1a83166fcb81998032d"),
    ];
    captures
        .into_iter()
        .try_for_each(|(capture, expected)| -> Result<()> {
            let key = cache_key(&capture.metadata.url)?;
            if key != expected {
                return Err(
                    "exact original production cache key differs; no endpoint mutation permitted"
                        .into(),
                );
            }
            io::write(
                &store.http_cache_dir().join(format!("{key}.body")),
                &capture.body,
            )?;
            io::write(
                &store.http_cache_dir().join(format!("{key}.meta.json")),
                &capture.metadata_bytes,
            )
        })?;
    io::write(&store.out_dir().join("schools.jsonl"), &inputs.schools)
}

fn save_sources(directory: &Path) -> Result<()> {
    io::directory(directory)?;
    let manifest = sources()
        .map(|(path, bytes)| -> Result<Value> {
            let name = path.replace('/', "__");
            io::write(&directory.join(&name), bytes)?;
            Ok(
                json!({"source_path": path, "artifact": name, "bytes": bytes.len(),
            "sha256": io::digest(bytes), "provenance": "compile-time embedded source snapshot"}),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    io::json(&directory.join("manifest.json"), &manifest)
}

fn sources() -> impl Iterator<Item = &'static (&'static str, &'static [u8])> {
    qualification_sources()
        .iter()
        .chain(collector_sources().iter())
        .chain(projection_sources().iter())
        .chain(retention_sources().iter())
}

fn qualification_sources() -> &'static [(&'static str, &'static [u8])] {
    &[
        (
            "examples/qualification_owned_projection.rs",
            include_bytes!("../qualification_owned_projection.rs"),
        ),
        (
            "examples/qualification_owned_projection/mod.rs",
            include_bytes!("mod.rs"),
        ),
        (
            "examples/qualification_owned_projection/input.rs",
            include_bytes!("input.rs"),
        ),
        (
            "examples/qualification_owned_projection/io.rs",
            include_bytes!("io.rs"),
        ),
        (
            "examples/qualification_owned_projection/measure.rs",
            include_bytes!("measure.rs"),
        ),
        (
            "examples/qualification_owned_projection/preserve.rs",
            include_bytes!("preserve.rs"),
        ),
    ]
}

fn collector_sources() -> &'static [(&'static str, &'static [u8])] {
    &[
        ("src/milesplit.rs", include_bytes!("../../src/milesplit.rs")),
        (
            "src/milesplit/fetch.rs",
            include_bytes!("../../src/milesplit/fetch.rs"),
        ),
        (
            "src/milesplit/wire.rs",
            include_bytes!("../../src/milesplit/wire.rs"),
        ),
        (
            "src/milesplit/results.rs",
            include_bytes!("../../src/milesplit/results.rs"),
        ),
        (
            "src/milesplit/results/run.rs",
            include_bytes!("../../src/milesplit/results/run.rs"),
        ),
    ]
}

fn projection_sources() -> &'static [(&'static str, &'static [u8])] {
    &[
        (
            "examples/qualification_owned_projection/inspect.rs",
            include_bytes!("inspect.rs"),
        ),
        (
            "src/milesplit/results/run/metadata.rs",
            include_bytes!("../../src/milesplit/results/run/metadata.rs"),
        ),
        (
            "src/milesplit/results/accumulator.rs",
            include_bytes!("../../src/milesplit/results/accumulator.rs"),
        ),
        (
            "src/milesplit/map.rs",
            include_bytes!("../../src/milesplit/map.rs"),
        ),
        (
            "src/milesplit/map_rows.rs",
            include_bytes!("../../src/milesplit/map_rows.rs"),
        ),
        (
            "src/milesplit/map/owned.rs",
            include_bytes!("../../src/milesplit/map/owned.rs"),
        ),
        (
            "src/milesplit/owned/mod.rs",
            include_bytes!("../../src/milesplit/owned/mod.rs"),
        ),
        (
            "src/milesplit/owned/effect.rs",
            include_bytes!("../../src/milesplit/owned/effect.rs"),
        ),
        (
            "src/milesplit/owned/parse.rs",
            include_bytes!("../../src/milesplit/owned/parse.rs"),
        ),
        (
            "src/milesplit/raw.rs",
            include_bytes!("../../src/milesplit/raw.rs"),
        ),
    ]
}

fn retention_sources() -> &'static [(&'static str, &'static [u8])] {
    &[
        ("src/net/cache.rs", include_bytes!("../../src/net/cache.rs")),
        (
            "src/net/execute.rs",
            include_bytes!("../../src/net/execute.rs"),
        ),
        (
            "census-store/src/read/mod.rs",
            include_bytes!("../../../census-store/src/read/mod.rs"),
        ),
        (
            "census-store/src/read/view/mod.rs",
            include_bytes!("../../../census-store/src/read/view/mod.rs"),
        ),
        (
            "census-store/src/keys.rs",
            include_bytes!("../../../census-store/src/keys.rs"),
        ),
    ]
}
