use anyhow::{bail, Context, Result};
use std::ffi::OsStr;
use std::fs::{self, DirEntry, ReadDir};
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy)]
pub(super) enum Selection {
    All,
    Rust,
}

#[derive(Clone, Copy)]
struct Limits {
    entries: usize,
    files: usize,
    depth: usize,
}

struct Frame {
    path: PathBuf,
    entries: ReadDir,
}

pub(super) fn run(root: &Path, skip: &[&str], selection: Selection) -> Result<Vec<PathBuf>> {
    let files = match selection {
        Selection::All => 1_000_000,
        Selection::Rust => 100_000,
    };
    run_with_limits(
        root,
        skip,
        selection,
        Limits {
            entries: 1_000_000,
            files,
            depth: 128,
        },
    )
}

fn run_with_limits(
    root: &Path,
    skip: &[&str],
    selection: Selection,
    limits: Limits,
) -> Result<Vec<PathBuf>> {
    let root = root_path(root);
    if matches!(selection, Selection::Rust) {
        refuse_root_ancestor_links(&root)?;
    }
    let metadata = fs::symlink_metadata(&root)
        .with_context(|| format!("reading root metadata for {}", root.display()))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!("source root must be a real directory: {}", root.display());
    }
    let mut frames = Vec::new();
    open_directory(root, &mut frames, limits.depth)?;
    let mut files = Vec::new();
    let mut visited = 0;
    while let Some(frame) = frames.last_mut() {
        let Some(entry) = frame.entries.next() else {
            drop(frames.pop());
            continue;
        };
        visited = admit(visited, limits.entries, "visited entry", &frame.path)?;
        let entry = entry.with_context(|| format!("listing {}", frame.path.display()))?;
        visit_entry(entry, skip, selection, limits, &mut frames, &mut files)?;
    }
    files.sort_unstable();
    Ok(files)
}

fn root_path(root: &Path) -> PathBuf {
    let mut components = root.components();
    match components.next_back() {
        Some(Component::Normal(name)) => components.as_path().join(name),
        _ => root.to_path_buf(),
    }
}

fn refuse_root_ancestor_links(root: &Path) -> Result<()> {
    let mut components = root.components().peekable();
    let mut prefix = PathBuf::new();
    let mut count = 0;
    while let Some(component) = components.next() {
        count = admit(count, 128, "root component", root)?;
        let additional = component
            .as_os_str()
            .len()
            .checked_add(1)
            .context("root prefix capacity overflow")?;
        prefix
            .try_reserve(additional)
            .context("reserving root prefix storage")?;
        prefix.push(component.as_os_str());
        if components.peek().is_none() {
            break;
        }
        let metadata = fs::symlink_metadata(&prefix)
            .with_context(|| format!("reading root ancestor metadata for {}", prefix.display()))?;
        if metadata.file_type().is_symlink() {
            bail!(
                "source root must be a real directory without symlinks: {}",
                prefix.display()
            );
        }
    }
    Ok(())
}

fn visit_entry(
    entry: DirEntry,
    skip: &[&str],
    selection: Selection,
    limits: Limits,
    frames: &mut Vec<Frame>,
    files: &mut Vec<PathBuf>,
) -> Result<()> {
    let name = entry.file_name();
    let excluded = name.to_str().is_some_and(|name| skip.contains(&name));
    if matches!(selection, Selection::Rust) && excluded {
        return Ok(());
    }
    let kind = entry
        .file_type()
        .with_context(|| format!("reading the type of {}", entry.path().display()))?;
    if matches!(selection, Selection::Rust) && kind.is_symlink() {
        bail!("source symlink refused: {}", entry.path().display());
    }
    if kind.is_dir() {
        if excluded {
            return Ok(());
        }
        return open_directory(entry.path(), frames, limits.depth);
    }
    if matches!(selection, Selection::Rust) {
        if Path::new(&name).extension() != Some(OsStr::new("rs")) {
            return Ok(());
        }
        if !kind.is_file() {
            bail!(
                "selected Rust source is not a regular file: {}",
                entry.path().display()
            );
        }
    }
    let path = entry.path();
    admit(files.len(), limits.files, "selected file", &path)?;
    files
        .try_reserve(1)
        .context("reserving selected file storage")?;
    files.push(path);
    Ok(())
}

fn open_directory(path: PathBuf, frames: &mut Vec<Frame>, depth: usize) -> Result<()> {
    admit(frames.len(), depth, "directory depth", &path)?;
    frames
        .try_reserve(1)
        .context("reserving directory frame storage")?;
    let entries = fs::read_dir(&path).with_context(|| format!("listing {}", path.display()))?;
    frames.push(Frame { path, entries });
    Ok(())
}

fn admit(current: usize, limit: usize, resource: &str, path: &Path) -> Result<usize> {
    let next = current
        .checked_add(1)
        .with_context(|| format!("{resource} counter overflow at {}", path.display()))?;
    if next > limit {
        bail!("{resource} limit {limit} exceeded at {}", path.display());
    }
    Ok(next)
}

#[cfg(test)]
#[path = "walk/tests.rs"]
mod tests;

#[cfg(all(test, unix))]
#[path = "walk/unix_tests.rs"]
mod unix_tests;
