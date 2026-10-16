use super::*;
use std::path::Component;

pub(super) const MAX_PATH_BYTES: usize = 4096;

pub(super) fn text(path: &Path) -> TestResult<&str> {
    let text = path.to_str().ok_or("handoff path must be UTF-8")?;
    if text.is_empty() || text.len() > MAX_PATH_BYTES || text.chars().any(char::is_control) {
        return Err("handoff path exceeds its bounded text grammar".into());
    }
    Ok(text)
}

pub(super) fn destination(path: &Path) -> TestResult<PathBuf> {
    let value = text(path)?;
    if !path.is_absolute()
        || value
            .split('/')
            .skip(1)
            .any(|part| part.is_empty() || part == "." || part == "..")
        || path
            .components()
            .any(|part| !matches!(part, Component::RootDir | Component::Normal(_)))
    {
        return Err("handoff destination must be an absolute normalized path".into());
    }
    absent(path)?;
    let parent = path
        .parent()
        .ok_or("handoff destination parent is absent")?
        .canonicalize()?;
    if !parent.is_dir() {
        return Err("handoff destination parent must be a directory".into());
    }
    let name = path
        .file_name()
        .ok_or("handoff destination name is absent")?;
    let resolved = parent.join(name);
    text(&resolved)?;
    absent(&resolved)?;
    Ok(resolved)
}

fn absent(path: &Path) -> TestResult {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
        Ok(_) => Err("handoff destination already exists".into()),
    }
}

pub(super) fn kept(path: &Path) -> TestResult<PathBuf> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("handoff kept directory must be a nonsymlink directory".into());
    }
    let kept = path.canonicalize()?;
    text(&kept)?;
    let checkout = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or("handoff checkout root is absent")?
        .canonicalize()?;
    if kept.starts_with(checkout) {
        return Err("handoff store must remain outside the checkout".into());
    }
    Ok(kept)
}

pub(super) fn relative(parent: &Path, name: &str) -> TestResult<PathBuf> {
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\') {
        return Err("handoff entry name contains traversal or a separator".into());
    }
    let separator = usize::from(!parent.as_os_str().is_empty());
    let bytes = parent
        .as_os_str()
        .len()
        .checked_add(name.len())
        .and_then(|n| n.checked_add(separator))
        .ok_or("handoff relative path size overflow")?;
    if bytes > MAX_PATH_BYTES {
        return Err("handoff relative path exceeds its byte bound".into());
    }
    let result = parent.join(name);
    text(&result)?;
    if result
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("handoff relative path contains traversal".into());
    }
    Ok(result)
}
