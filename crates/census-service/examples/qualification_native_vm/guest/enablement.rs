use super::process;
use anyhow::{ensure, Context, Result};
use std::fs::{self, File};
use std::path::Path;
use std::process::Command;

#[cfg(test)]
mod tests;

pub(super) fn enable(root: &Path) -> Result<()> {
    process::command(
        root,
        "enable",
        Command::new("/usr/bin/systemctl").args(["enable", "--now", "qualification.service"]),
        100,
    )?;
    persist(
        Path::new("/etc/systemd/system/multi-user.target.wants/qualification.service"),
        Path::new("/etc/systemd/system/qualification.service"),
    )
}

fn persist(link: &Path, unit: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(link)
        .with_context(|| format!("reading boot enablement link {}", link.display()))?;
    ensure!(
        metadata.file_type().is_symlink(),
        "boot enablement is not a genuine symlink: {}",
        link.display()
    );
    let resolved = fs::canonicalize(link)
        .with_context(|| format!("resolving boot enablement link {}", link.display()))?;
    let actual = fs::canonicalize(unit)
        .with_context(|| format!("resolving owned qualification unit {}", unit.display()))?;
    ensure!(
        resolved == actual,
        "boot enablement resolves to foreign unit: {}; expected {}",
        resolved.display(),
        actual.display()
    );
    let directory = link.parent().context("boot enablement directory absent")?;
    let parent = directory
        .parent()
        .context("systemd unit directory absent")?;
    File::open(directory)
        .and_then(|directory| directory.sync_all())
        .with_context(|| {
            format!(
                "persisting boot enablement directory {}",
                directory.display()
            )
        })?;
    File::open(parent)
        .and_then(|parent| parent.sync_all())
        .with_context(|| format!("persisting systemd unit directory {}", parent.display()))
}
