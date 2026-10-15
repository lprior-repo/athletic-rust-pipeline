use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

pub(super) fn output(store: &Path, requested: &Path) -> Result<PathBuf> {
    let store =
        std::fs::canonicalize(store).context("selected-publication requires an existing store")?;
    let name = requested
        .file_name()
        .context("publication output must name a new directory")?;
    let parent = match requested
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        Some(parent) => parent,
        None => Path::new("."),
    };
    let parent = std::fs::canonicalize(parent).context("publication parent must already exist")?;
    let path = parent.join(name);
    anyhow::ensure!(
        !path.starts_with(&store),
        "qualification publication must be outside the source store"
    );
    match std::fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path),
        Err(error) => Err(error.into()),
        Ok(_) => anyhow::bail!("publication output already exists; refusing to overwrite"),
    }
}

pub(super) struct Scratch(PathBuf);

impl Scratch {
    pub(super) fn create(output: &Path) -> Result<Self> {
        let parent = output.parent().context("publication parent")?;
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let path = parent.join(format!(".selected-proof-{}-{stamp}", std::process::id()));
        std::fs::create_dir(&path).context("creating new owned proof staging directory")?;
        Ok(Self(path))
    }

    pub(super) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if let Err(error) = std::fs::remove_dir_all(&self.0) {
            tracing::warn!(path = %self.0.display(), %error, "could not remove owned selected proof staging");
        }
    }
}
