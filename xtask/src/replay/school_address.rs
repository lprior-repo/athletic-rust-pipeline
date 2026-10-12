use std::path::Path;

use anyhow::{bail, Result};
use census_service::school_address::{replay, verify_current};

pub(super) fn run(manifest: Option<&Path>, out: Option<&Path>) -> Result<()> {
    let default_manifest = crate::paths::fixtures_dir().join("school_address_replay/manifest.json");
    let default_out = crate::paths::repo_root().join("target/address-replay");
    let manifest = manifest.map_or(default_manifest.as_path(), core::convert::identity);
    let out = out.map_or(default_out.as_path(), core::convert::identity);
    let first = replay::run(manifest, out)?;
    let before = verify_current(out)?;
    let second = replay::run(manifest, out)?;
    let after = verify_current(out)?;
    for name in [
        "manifest.json",
        "school_directory.json",
        "school_directory.csv",
        "pipeline_report.json",
        "baseline.json",
        "update_ledger.json",
    ] {
        if before.artifact(name)? != after.artifact(name)? {
            bail!("address replay is not deterministic: {name} differs");
        }
    }
    if first.generation_digest() != second.generation_digest() {
        bail!("address replay generation digest differs");
    }
    println!(
        "school_address replay: two byte-identical generations {}",
        first.generation_digest()
    );
    println!(
        "capture provenance: {}",
        serde_json::to_string(first.overrides())?
    );
    Ok(())
}
