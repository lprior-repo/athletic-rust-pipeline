use anyhow::{Context, Result};
use std::path::{Component, Path, PathBuf};

pub(super) fn destination(original: &Path, requested: &Path) -> Result<PathBuf> {
    anyhow::ensure!(
        std::fs::symlink_metadata(original)?.file_type().is_dir(),
        "original publication root must be a real directory"
    );
    let original = std::fs::canonicalize(original)?;
    anyhow::ensure!(
        matches!(
            requested.components().next_back(),
            Some(Component::Normal(_))
        ),
        "derived publication root must end in a directory name"
    );
    let name = requested.file_name().context("derived directory name")?;
    let parent = requested
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or(Path::new("."), std::convert::identity);
    let parent = std::fs::canonicalize(parent).context("derived publication parent")?;
    let destination = parent.join(name);
    anyhow::ensure!(
        original.parent() == Some(parent.as_path()) && destination != original,
        "derived publication must be a different sibling of the original root"
    );
    require_destination(&destination)?;
    Ok(destination)
}

fn require_destination(destination: &Path) -> Result<()> {
    match std::fs::symlink_metadata(destination) {
        Ok(metadata) => anyhow::ensure!(
            metadata.file_type().is_dir(),
            "derived publication root must be a real directory"
        ),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    }
    if std::fs::read_dir(destination)?
        .next()
        .transpose()?
        .is_none()
    {
        return Ok(());
    }
    anyhow::ensure!(
        std::fs::symlink_metadata(destination.join("generations"))
            .context("existing derived root must contain a publication layout")?
            .file_type()
            .is_dir(),
        "derived generations must be a real directory"
    );
    Ok(())
}

#[cfg(test)]
mod tests;
