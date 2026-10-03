use super::{Configuration, LIMIT, MARKER};
use crate::qualification_native_vm::artifacts;
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::fs::{File, Metadata, OpenOptions};
use std::io::{ErrorKind, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

pub(super) fn pending(path: &Path) -> Result<PathBuf> {
    let name = path
        .file_name()
        .context("native artifact filename absent")?;
    let mut name = name.to_os_string();
    name.push(".pending");
    Ok(path.with_file_name(name))
}

pub(super) fn private_parent(path: &Path) -> Result<Metadata> {
    let parent = path.parent().context("native artifact parent absent")?;
    let metadata = std::fs::symlink_metadata(parent)?;
    ensure!(
        parent.is_absolute()
            && parent.canonicalize()? == parent
            && metadata.is_dir()
            && metadata.uid() == std::fs::metadata("/proc/self")?.uid()
            && metadata.mode() & 0o777 == 0o700,
        "native boundary requires an owned private nonsymlink directory: {}",
        parent.display()
    );
    Ok(metadata)
}

fn metadata(path: &Path) -> Result<Option<Metadata>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("inspecting {}", path.display())),
    }
}

fn absent(path: &Path) -> Result<()> {
    ensure!(
        metadata(path)?.is_none(),
        "native boundary refuses reused or conflicting artifact: {}",
        path.display()
    );
    Ok(())
}

pub(super) fn publish_config(path: &Path, configuration: &Configuration) -> Result<()> {
    private_parent(path)?;
    let marker = path.with_file_name(MARKER);
    let pending = pending(path)?;
    let marker_pending = self::pending(&marker)?;
    [
        path,
        pending.as_path(),
        marker.as_path(),
        marker_pending.as_path(),
    ]
    .into_iter()
    .try_for_each(absent)?;
    let bytes = serde_json::to_vec(configuration)?;
    ensure!(
        u64::try_from(bytes.len())? <= LIMIT,
        "native config exceeds 4096 bytes"
    );
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(&pending)
        .context("creating private native config pending file")?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    verify_owned(&pending, &file)?;
    std::fs::hard_link(&pending, path)
        .context("publishing native config without overwriting authority")?;
    verify_owned(path, &file)?;
    let parent = File::open(path.parent().context("native config parent absent")?)?;
    parent.sync_all()?;
    verify_owned(&pending, &file)?;
    std::fs::remove_file(&pending).context("removing owned native config pending link")?;
    parent.sync_all()?;
    Ok(())
}

fn open_read(path: &Path) -> Result<File> {
    ensure!(
        cfg!(target_os = "linux"),
        "native boundary filesystem requires Linux"
    );
    OpenOptions::new()
        .read(true)
        .custom_flags(0o400000 | 0o4000)
        .open(path)
        .with_context(|| {
            format!(
                "opening native artifact without symlink/blocking: {}",
                path.display()
            )
        })
}

fn verify_owned(path: &Path, file: &File) -> Result<()> {
    let current = std::fs::symlink_metadata(path)?;
    let owned = file.metadata()?;
    ensure!(
        current.is_file() && current.dev() == owned.dev() && current.ino() == owned.ino(),
        "native config publication no longer owns {}",
        path.display()
    );
    Ok(())
}

fn unchanged(left: &Metadata, right: &Metadata) -> bool {
    left.dev() == right.dev()
        && left.ino() == right.ino()
        && left.len() == right.len()
        && left.mode() == right.mode()
        && left.uid() == right.uid()
        && left.nlink() == right.nlink()
        && left.mtime() == right.mtime()
        && left.mtime_nsec() == right.mtime_nsec()
        && left.ctime() == right.ctime()
        && left.ctime_nsec() == right.ctime_nsec()
}

fn regular(path: &Path, metadata: &Metadata, owner: &Metadata) -> Result<()> {
    ensure!(
        metadata.is_file()
            && metadata.uid() == owner.uid()
            && metadata.mode() & 0o777 == 0o600
            && metadata.nlink() == 1,
        "native boundary artifact is not an owned private regular single-link file: {}",
        path.display()
    );
    ensure!(
        metadata.len() <= LIMIT,
        "native boundary artifact exceeds 4096 bytes: {}",
        path.display()
    );
    Ok(())
}

pub(super) fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    let owner = private_parent(path)?;
    let Some(before) = metadata(path)? else {
        return Ok(None);
    };
    regular(path, &before, &owner)?;
    let file = open_read(path)?;
    let opened = file.metadata()?;
    regular(path, &opened, &owner)?;
    ensure!(
        unchanged(&before, &opened),
        "native artifact changed during open"
    );
    let mut bytes = Vec::new();
    bytes.try_reserve_exact(usize::try_from(LIMIT.saturating_add(1))?)?;
    (&file)
        .take(LIMIT.saturating_add(1))
        .read_to_end(&mut bytes)?;
    let after = std::fs::symlink_metadata(path)?;
    regular(path, &after, &owner)?;
    ensure!(
        unchanged(&before, &after)
            && unchanged(&after, &file.metadata()?)
            && u64::try_from(bytes.len())? == before.len(),
        "native artifact changed or exceeded its bounded read"
    );
    Ok(Some(bytes))
}

pub(super) fn read_required(path: &Path) -> Result<Vec<u8>> {
    read_optional(path)?
        .with_context(|| format!("native boundary artifact missing: {}", path.display()))
}

pub(super) fn published_marker(path: &Path) -> Result<Option<Vec<u8>>> {
    let owner = private_parent(path)?;
    if let Some(pending) = metadata(&pending(path)?)? {
        ensure!(
            pending.is_file()
                && pending.uid() == owner.uid()
                && pending.mode() & 0o777 == 0o600
                && (1..=2).contains(&pending.nlink())
                && pending.len() <= LIMIT,
            "native marker pending artifact is unsafe or exceeds its publication budget"
        );
        return Ok(None);
    }
    let Some(before) = metadata(path)? else {
        return Ok(None);
    };
    let Some(bytes) = read_optional(path)? else {
        return Ok(None);
    };
    let file = open_read(path)?;
    let opened = file.metadata()?;
    regular(path, &opened, &owner)?;
    ensure!(
        unchanged(&before, &opened),
        "native marker changed before durability confirmation"
    );
    file.sync_all()
        .context("syncing reached native reservation marker")?;
    File::open(path.parent().context("native marker parent absent")?)?
        .sync_all()
        .context("syncing complete native marker publication")?;
    ensure!(
        unchanged(&opened, &std::fs::symlink_metadata(path)?),
        "native marker changed during durability confirmation"
    );
    Ok(Some(bytes))
}

pub(super) fn evidence(path: &Path, bytes: &[u8]) -> Result<Value> {
    Ok(
        json!({"path":path,"sha256":artifacts::sha(bytes),"bytes":bytes,"content":serde_json::from_slice::<Value>(bytes)?}),
    )
}
