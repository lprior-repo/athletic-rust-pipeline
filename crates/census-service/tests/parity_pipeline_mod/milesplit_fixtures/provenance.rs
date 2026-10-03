use std::collections::BTreeSet;
use std::io::Read;
use std::path::Path;

use anyhow::{ensure, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use super::{RAW_FILES, TROY_URL};
use crate::common;

#[derive(Deserialize)]
struct Manifest {
    captures: Vec<Binding>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    projection: String,
    raw_path: String,
    raw_sha256: String,
    published_url: String,
}

pub(super) fn validate(body: &str) -> Result<()> {
    let manifest: Manifest =
        serde_json::from_str(body).context("decoding raw projection bindings")?;
    check!(eq; manifest.captures.len(), RAW_FILES.len());
    let names: BTreeSet<_> = manifest
        .captures
        .iter()
        .map(|row| row.projection.as_str())
        .collect();
    check!(eq; names, BTreeSet::from(RAW_FILES));
    manifest.captures.iter().try_for_each(validate_binding)
}

fn validate_binding(binding: &Binding) -> Result<()> {
    let (rsid, key, digest, acquired_at) = match binding.projection.as_str() {
        "troy_725218_rs1266814_raw_projection.html" => (
            "1266814",
            "c7791d114036f1a83166fcb81998032d",
            "729850b479e5782aa3e4ade7740cd46b8ffd8f35db79a873abd4ed0b3fb0f6cf",
            "2026-09-28T10:11:24Z",
        ),
        "troy_725218_rs1266815_raw_projection.html" => (
            "1266815",
            "f1273a7073688debd2ea7b3c0b12cb9c",
            "d9bcb109b52d5c54fddc1bede35a9ba6c4496d0c717d99bf8052113012165efd",
            "2026-09-28T10:11:13Z",
        ),
        name => anyhow::bail!("unsupported provenance projection {name}"),
    };
    check!(eq; binding.published_url, format!("{TROY_URL}/{rsid}/raw"));
    check!(eq; binding.raw_sha256, digest);
    check!(eq; binding.raw_path,
    format!("var/retained-pr-correction-20261001/store/http/{key}.body"));
    let projection = common::fixture("milesplit", &binding.projection)?;
    super::raw::validate_projection(&projection, rsid == "1266815")?;
    let original = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(&binding.raw_path);
    if original
        .try_exists()
        .context("checking retained source capture")?
    {
        validate_original(&original, binding, acquired_at, projection.as_bytes())?;
    }
    Ok(())
}

fn validate_original(
    path: &Path,
    binding: &Binding,
    acquired_at: &str,
    projection: &[u8],
) -> Result<()> {
    let bytes = read_bounded(path)?;
    check!(eq; format!("{:x}", Sha256::digest(&bytes)), binding.raw_sha256);
    let meta: serde_json::Value =
        serde_json::from_slice(&read_bounded(&path.with_extension("meta.json"))?)?;
    check!(eq; meta["url"], binding.published_url);
    check!(eq; meta["content_digest"], binding.raw_sha256);
    check!(eq; meta["bytes"], bytes.len());
    check!(eq; meta["status"], 200);
    check!(eq; meta["method"], "GET");
    check!(eq; meta["fetched_at"], acquired_at);
    let mut selected = Vec::new();
    let lines: Vec<_> = bytes.split_inclusive(|byte| *byte == b'\n').collect();
    for (start, end) in [(42usize, 63usize), (208, 208), (712, 719)] {
        let range = lines
            .get(start - 1..end)
            .context("raw capture does not contain projection range")?;
        range
            .iter()
            .for_each(|line| selected.extend_from_slice(line));
    }
    selected.extend_from_slice(b"</pre>\n");
    check!(eq; projection, selected,
    "projection altered retained source bytes");
    Ok(())
}

pub(super) fn validate_relay_origin(projection: &serde_json::Value) -> Result<()> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../var/retained-pr-correction-20261001/store/http/52e0b5d61b6c7de90be2a35dd5fc3f42.body",
    );
    if !path
        .try_exists()
        .context("checking retained relay source capture")?
    {
        return Ok(());
    }
    let bytes = read_bounded(&path)?;
    let provenance = &projection["provenance"];
    check!(eq; provenance["capture_sha256"],
    format!("{:x}", Sha256::digest(&bytes)));
    let meta: serde_json::Value =
        serde_json::from_slice(&read_bounded(&path.with_extension("meta.json"))?)?;
    check!(eq; meta["url"], provenance["source_url"]);
    check!(eq; meta["content_digest"], provenance["capture_sha256"]);
    check!(eq; meta["fetched_at"], provenance["fetched_at"]);
    let original: serde_json::Value = serde_json::from_slice(&bytes)?;
    let rows = original["data"]
        .as_array()
        .context("relay origin has no data array")?;
    let selected: Vec<_> = [142, 146, 460]
        .into_iter()
        .map(|index| rows.get(index).context("relay origin locator missing"))
        .collect::<Result<_>>()?;
    let projected = projection["data"]
        .as_array()
        .context("relay projection has no data array")?;
    check!(eq; projected.iter().collect::<Vec<_>>(), selected);
    Ok(())
}

fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    const LIMIT: u64 = 1024 * 1024;
    let file = std::fs::File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .with_context(|| format!("reading {}", path.display()))?;
    ensure!(
        u64::try_from(bytes.len())? <= LIMIT,
        "capture exceeds one MiB: {}",
        path.display()
    );
    Ok(bytes)
}
