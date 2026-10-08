use super::super::Throughput;
use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::collections::{btree_map::Entry, BTreeMap};
use std::ffi::OsStr;
use std::path::Path;

#[derive(Deserialize)]
struct Benchmark {
    full_id: String,
    throughput: Option<Throughput<u64>>,
}

pub(super) fn read(root: &Path) -> Result<BTreeMap<String, Option<Throughput<u64>>>> {
    let mut pending = vec![root.to_owned()];
    let mut seen = 0usize;
    let mut benchmarks = BTreeMap::new();
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)
            .with_context(|| format!("reading Criterion metadata {}", directory.display()))?
        {
            let entry = entry?;
            seen = seen
                .checked_add(1)
                .context("Criterion metadata entry count overflow")?;
            if seen > 4_096 {
                bail!("Criterion metadata exceeds 4096 entries");
            }
            let path = entry.path();
            let kind = entry.file_type()?;
            if kind.is_symlink() {
                bail!(
                    "Criterion metadata must not contain symlinks: {}",
                    path.display()
                );
            }
            if kind.is_dir() {
                pending.push(path);
            } else if path.file_name() == Some(OsStr::new("benchmark.json"))
                && directory.file_name() == Some(OsStr::new("new"))
            {
                let data: Benchmark = serde_json::from_slice(&std::fs::read(&path)?)
                    .with_context(|| format!("parsing Criterion metadata {}", path.display()))?;
                insert(&mut benchmarks, data)?;
            }
        }
    }
    if benchmarks.is_empty() {
        bail!("Criterion produced no benchmark metadata");
    }
    Ok(benchmarks)
}

fn insert(
    benchmarks: &mut BTreeMap<String, Option<Throughput<u64>>>,
    data: Benchmark,
) -> Result<()> {
    if data.full_id.is_empty() {
        bail!("Criterion benchmark ID is empty");
    }
    let throughput = data.throughput.with_context(|| {
        format!(
            "Criterion benchmark {} must declare a throughput amount",
            data.full_id
        )
    })?;
    if throughput.value() == 0 {
        bail!("Criterion throughput must be positive for {}", data.full_id);
    }
    match benchmarks.entry(data.full_id) {
        Entry::Vacant(entry) => {
            entry.insert(Some(throughput));
        }
        Entry::Occupied(entry) => bail!("duplicate Criterion benchmark ID: {}", entry.key()),
    }
    Ok(())
}

#[cfg(test)]
#[path = "metadata/tests.rs"]
mod tests;
