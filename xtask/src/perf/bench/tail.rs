use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct Samples {
    iters: Vec<f64>,
    times: Vec<f64>,
}

pub(super) fn read(root: &Path) -> Result<BTreeMap<String, f64>> {
    let mut tails = BTreeMap::new();
    for path in sample_paths(root)? {
        let parent = path.parent().context("sample path has no parent")?;
        let metadata: serde_json::Value =
            serde_json::from_slice(&std::fs::read(parent.join("benchmark.json"))?)?;
        let id = metadata
            .get("full_id")
            .and_then(|id| id.as_str())
            .context("sample benchmark has no ID")?;
        let samples: Samples = serde_json::from_slice(&std::fs::read(path)?)?;
        if tails
            .insert(id.to_owned(), maximum_sample(&samples)?)
            .is_some()
        {
            bail!("duplicate sample benchmark ID: {id}");
        }
    }
    Ok(tails)
}

fn sample_paths(root: &Path) -> Result<Vec<PathBuf>> {
    let mut pending = vec![root.to_owned()];
    let mut paths = Vec::new();
    let mut seen = 0usize;
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)? {
            seen = seen.checked_add(1).context("sample entry count overflow")?;
            if seen > 4_096 {
                bail!("Criterion samples exceed 4096 entries");
            }
            let entry = entry?;
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                bail!("Criterion sample path must not be a symlink");
            }
            if kind.is_dir() {
                pending.push(entry.path());
            } else if entry.file_name() == OsStr::new("sample.json")
                && directory.file_name() == Some(OsStr::new("new"))
            {
                paths.push(entry.path());
            }
        }
    }
    Ok(paths)
}

fn maximum_sample(samples: &Samples) -> Result<f64> {
    if samples.iters.is_empty() || samples.iters.len() != samples.times.len() {
        bail!("Criterion sample iterations and times must have equal nonempty lengths");
    }
    let mut maximum = 0.0f64;
    for (&iterations, &nanos) in samples.iters.iter().zip(&samples.times) {
        let seconds = nanos / iterations / 1e9;
        if !iterations.is_finite()
            || iterations <= 0.0
            || !nanos.is_finite()
            || nanos <= 0.0
            || !seconds.is_finite()
            || seconds <= 0.0
        {
            bail!("Criterion samples must be finite and positive");
        }
        maximum = maximum.max(seconds);
    }
    Ok(maximum)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tail_preserves_slowest_sample_instead_of_mean() -> Result<()> {
        let samples = Samples {
            iters: vec![1.0, 2.0, 4.0],
            times: vec![1e9, 6e9, 8e9],
        };
        check!(eq; maximum_sample(&samples)?, 3.0);
        Ok(())
    }

    #[test]
    fn malformed_tail_samples_cannot_pass() -> Result<()> {
        for invalid in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            check!(maximum_sample(&Samples {
                iters: vec![invalid],
                times: vec![1.0]
            })
            .is_err());
            check!(maximum_sample(&Samples {
                iters: vec![1.0],
                times: vec![invalid]
            })
            .is_err());
        }
        check!(maximum_sample(&Samples {
            iters: vec![],
            times: vec![]
        })
        .is_err());
        check!(maximum_sample(&Samples {
            iters: vec![1.0],
            times: vec![]
        })
        .is_err());
        Ok(())
    }
}
