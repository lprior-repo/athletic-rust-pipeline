use crate::{
    coverage::{self, RunManifest},
    search_cache,
};
use anyhow::{Context, Result};
use std::{
    fs::{self, File, OpenOptions},
    path::Path,
};

pub fn import(input: &Path, config: &Path, output: &Path, donor: &Path, scope: &str) -> Result<()> {
    let lock = OpenOptions::new()
        .write(true)
        .open(donor.join("run.lock"))
        .context("opening deterministic donor lock")?;
    lock.try_lock()
        .context("deterministic donor is still running; stop it before review")?;
    let manifest: RunManifest =
        serde_json::from_slice(&fs::read(donor.join("run-manifest.json"))?)?;
    let expected = coverage::run_fingerprint(input, config, scope)?;
    if manifest.fingerprint != expected || manifest.scope != scope {
        anyhow::bail!("donor workbook, configuration, scope or analysis schema is incompatible");
    }
    let destination = output.join("search-cache.jsonl");
    if destination.exists() {
        return Ok(());
    }
    let source = donor.join("search-cache.jsonl");
    // Validate committed JSONL and repair only a torn suffix while holding the donor lock.
    let _records = search_cache::load_latest(&source)?;
    if !source.exists() {
        return Ok(());
    }
    let temporary = output.join("search-cache.import.tmp");
    fs::copy(&source, &temporary).context("copying verified discovery cache")?;
    File::open(&temporary)?.sync_all()?;
    fs::rename(&temporary, destination)?;
    File::open(output)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incompatible_donor_cannot_supply_search_evidence() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let donor = dir.path().join("donor");
        let output = dir.path().join("output");
        fs::create_dir_all(&donor)?;
        fs::create_dir_all(&output)?;
        let input = dir.path().join("input");
        let config = dir.path().join("config");
        fs::write(&input, "population one")?;
        fs::write(&config, "configuration")?;
        File::create(donor.join("run.lock"))?;
        coverage::bind_run(&input, &config, &donor, "first-worksheet:deterministic")?;
        fs::write(&input, "population two")?;
        assert!(import(
            &input,
            &config,
            &output,
            &donor,
            "first-worksheet:deterministic"
        )
        .is_err());
        assert!(!output.join("search-cache.jsonl").exists());
        Ok(())
    }
}
