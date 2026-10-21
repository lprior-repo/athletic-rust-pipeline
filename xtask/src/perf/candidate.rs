use super::{baseline, compare, PerfBaseline};
use anyhow::{bail, Context, Result};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

fn absolute_destination(path: &Path) -> Result<PathBuf> {
    let name = path
        .file_name()
        .context("performance output requires a file name")?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .map_or(Path::new("."), core::convert::identity);
    Ok(fs::canonicalize(parent)
        .with_context(|| {
            format!(
                "resolving performance output directory {}",
                parent.display()
            )
        })?
        .join(name))
}

fn prepare_output(output: &Path) -> Result<PathBuf> {
    let output = absolute_destination(output)?;
    if output == absolute_destination(&baseline::baseline_path())? {
        bail!("candidate output must not replace the canonical performance baseline");
    }
    match fs::symlink_metadata(&output) {
        Ok(_) => bail!("candidate output already exists: {}", output.display()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(output),
        Err(error) => {
            Err(error).with_context(|| format!("checking candidate output {}", output.display()))
        }
    }
}

pub(super) fn record(output: &Path, measure: impl FnOnce() -> Result<PerfBaseline>) -> Result<()> {
    let output = prepare_output(output)?;
    let measured = measure()?;
    compare::compare_environment(&measured.metadata, &measured.metadata)?;
    compare::validate_baseline(&measured)?;
    let json = serde_json::to_vec_pretty(&measured).context("serializing performance candidate")?;
    let parent = output
        .parent()
        .context("candidate output directory is missing")?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(&json)?;
    temporary.as_file().sync_all()?;
    temporary.persist_noclobber(&output).with_context(|| {
        format!(
            "installing performance candidate without replacement: {}",
            output.display()
        )
    })?;
    println!("wrote performance candidate to {}", output.display());
    Ok(())
}

pub(super) fn compare_files(
    baseline_path: &Path,
    candidate_path: &Path,
    tolerance: f64,
    reason: Option<&str>,
) -> Result<()> {
    compare::validate_tolerance(tolerance)?;
    let baseline_path = fs::canonicalize(baseline_path)
        .with_context(|| format!("resolving baseline {}", baseline_path.display()))?;
    let candidate_path = fs::canonicalize(candidate_path)
        .with_context(|| format!("resolving candidate {}", candidate_path.display()))?;
    if baseline_path == candidate_path {
        bail!("baseline and candidate must be different performance artifacts");
    }
    let baseline = baseline::load_baseline_from(&baseline_path)?;
    let candidate = baseline::load_baseline_from(&candidate_path)?;
    if serde_json::to_vec(&baseline)? == serde_json::to_vec(&candidate)? {
        bail!("baseline and candidate contain the same performance artifact");
    }
    compare::compare_environment(&baseline.metadata, &candidate.metadata)?;
    compare::validate_baseline(&baseline)?;
    compare::validate_baseline(&candidate)?;
    if let Some(reason) = reason {
        println!("artifact comparison reason: {reason}");
    }
    println!(
        "comparing supplied artifacts only: baseline={} candidate={}",
        baseline_path.display(),
        candidate_path.display()
    );
    compare::check_throughput(&baseline, &candidate.groups, tolerance)
}
