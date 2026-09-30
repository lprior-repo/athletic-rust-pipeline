use super::generation_error::io;
use super::manifest::{self, Manifest};
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CommitPoint {
    BeforeDirectory,
    BeforePointer,
    AfterPointer,
}

pub(super) fn publish(
    out: &Path,
    files: BTreeMap<String, Vec<u8>>,
    manifest: &Manifest,
) -> Result<()> {
    publish_with(out, files, manifest, |_| Ok(()))
}

pub(super) fn publish_with(
    out: &Path,
    files: BTreeMap<String, Vec<u8>>,
    manifest: &Manifest,
    fault: impl Fn(CommitPoint) -> Result<()>,
) -> Result<()> {
    std::fs::create_dir_all(out).with_context(|| format!("creating {}", out.display()))?;
    let generations = out.join("generations");
    std::fs::create_dir_all(&generations).map_err(|source| io(&generations, source))?;
    let stage = out.join(format!(".staging.{}", std::process::id()));
    std::fs::create_dir(&stage).map_err(|source| io(&stage, source))?;
    let result = stage_and_commit(out, &stage, &files, manifest, &fault);
    if stage.exists() {
        std::fs::remove_dir_all(&stage)
            .with_context(|| format!("removing incomplete staged siblings {}", stage.display()))?;
    }
    result
}

fn stage_and_commit(
    out: &Path,
    stage: &Path,
    files: &BTreeMap<String, Vec<u8>>,
    manifest: &Manifest,
    fault: &impl Fn(CommitPoint) -> Result<()>,
) -> Result<()> {
    for (name, bytes) in files {
        write_synced(&stage.join(name), bytes)?;
    }
    write_synced(&stage.join("manifest.json"), &serde_json::to_vec(manifest)?)?;
    sync_directory(stage)?;
    manifest::verify_staged(stage)?;
    let prefix = manifest
        .generation_digest
        .get(..16)
        .context("invalid generation digest")?;
    let target = out.join("generations").join(prefix);
    let temporary = prepare_pointer(out, prefix)?;
    let result = commit(out, stage, &target, &temporary, fault);
    if temporary.exists() || std::fs::symlink_metadata(&temporary).is_ok() {
        std::fs::remove_file(&temporary).map_err(|source| io(&temporary, source))?;
    }
    result.with_context(|| format!("publication failed at generation {}; incomplete staging {} is removed; a complete unreferenced generation is retained; inspect current to determine whether commit occurred", target.display(), stage.display()))
}

fn commit(
    out: &Path,
    stage: &Path,
    target: &Path,
    temporary: &Path,
    fault: &impl Fn(CommitPoint) -> Result<()>,
) -> Result<()> {
    fault(CommitPoint::BeforeDirectory)?;
    if target.exists() {
        let previous = manifest::verify_directory(out, target)?;
        let encoded = manifest::load(&stage.join("manifest.json"))?;
        if previous.artifact("manifest.json")? != encoded {
            bail!("generation digest prefix collision at {}", target.display());
        }
    } else {
        std::fs::rename(stage, target).map_err(|source| io(target, source))?;
    }
    sync_directory(&out.join("generations"))?;
    sync_directory(out)?;
    manifest::verify_directory(out, target)?;
    fault(CommitPoint::BeforePointer)?;
    std::fs::rename(temporary, out.join("current"))
        .map_err(|source| io(&out.join("current"), source))?;
    sync_directory(out)?;
    fault(CommitPoint::AfterPointer)?;
    Ok(())
}

fn prepare_pointer(out: &Path, prefix: &str) -> Result<PathBuf> {
    let temporary = out.join(format!(".current.{}.tmp", std::process::id()));
    std::os::unix::fs::symlink(Path::new("generations").join(prefix), &temporary)
        .map_err(|source| io(&temporary, source))?;
    Ok(temporary)
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|source| io(path, source))?;
    file.write_all(bytes).map_err(|source| io(path, source))?;
    file.sync_all().map_err(|source| io(path, source))?;
    Ok(())
}

fn sync_directory(path: &Path) -> Result<()> {
    std::fs::File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|source| io(path, source))?;
    Ok(())
}

#[cfg(test)]
#[path = "generation_tests.rs"]
mod tests;
