use super::TestResult;
use std::env;
use std::path::{Path, PathBuf};

const RETENTION_ENV: &str = "CENSUS_CONTROLLED_ELIGIBLE_IDENTITY_RETENTION_DIR";

pub(super) fn root(temporary: &Path) -> TestResult<PathBuf> {
    let Some(env_dir) = env::var_os(RETENTION_ENV) else {
        return Ok(temporary.join("publication"));
    };
    let requested = PathBuf::from(env_dir);
    if !requested.is_absolute() {
        return Err("CENSUS_CONTROLLED_ELIGIBLE_IDENTITY_RETENTION_DIR must be absolute".into());
    }
    let filename = requested
        .file_name()
        .ok_or("CENSUS_CONTROLLED_ELIGIBLE_IDENTITY_RETENTION_DIR must have a filename")?;
    let parent = requested
        .parent()
        .ok_or("CENSUS_CONTROLLED_ELIGIBLE_IDENTITY_RETENTION_DIR parent")?;
    let canonical_parent = parent.canonicalize()?;
    if !canonical_parent.is_dir() {
        return Err("retention dir parent must be an existing directory".into());
    }
    let temp = env::temp_dir().canonicalize()?;
    if !canonical_parent.starts_with(&temp) {
        return Err("retention dir parent must be inside temp dir".into());
    }
    let reconstructed = canonical_parent.join(filename);
    if requested != reconstructed {
        return Err("retention dir must not contain .. or symlink ancestors".into());
    }
    std::fs::create_dir(&reconstructed)?;
    Ok(reconstructed)
}

pub(super) fn receipt(prior: &Path, current: &Path) -> TestResult<()> {
    if env::var_os(RETENTION_ENV).is_none() {
        return Ok(());
    }
    let prior_meta = prior.symlink_metadata()?;
    if prior_meta.file_type().is_symlink() || !prior_meta.file_type().is_file() {
        return Err("prior workbook must be an existing regular file".into());
    }
    let current_meta = current.symlink_metadata()?;
    if current_meta.file_type().is_symlink() || !current_meta.file_type().is_file() {
        return Err("current workbook must be an existing regular file".into());
    }
    let payload = serde_json::json!({
        "schema": "controlled-eligible-identity-retention/v1",
        "synthetic": true,
        "pre_split_workbook": prior.to_str().ok_or("prior workbook path is not UTF-8")?,
        "post_split_workbook": current.to_str().ok_or("current workbook path is not UTF-8")?
    });
    eprintln!("{payload}");
    Ok(())
}
