use anyhow::{bail, Context, Result};
use std::{fs, path::Path};

pub(super) fn reject_destination(
    destination: &Path,
    parent: &Path,
    source: &Path,
    frozen: &Path,
) -> Result<()> {
    match fs::symlink_metadata(destination) {
        Ok(_) => bail!("export destination already exists or is a symlink"),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error).context("checking export destination"),
    }
    let name = destination
        .file_name()
        .context("export destination has no filename")?;
    let candidate = parent.join(name);
    [source, frozen].iter().try_for_each(|path| {
        if fs::canonicalize(path).is_ok_and(|canonical| canonical == candidate) {
            bail!("export destination must not replace the source workbook");
        }
        Ok(())
    })
}
