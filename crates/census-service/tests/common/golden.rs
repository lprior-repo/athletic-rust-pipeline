use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

fn golden_dir() -> Result<PathBuf> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests");
    if !dir.is_dir() {
        bail!("missing test directory at {}", dir.display());
    }
    Ok(dir.join("golden"))
}

pub fn assert_golden<T: serde::Serialize>(name: &str, value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)
        .with_context(|| format!("serializing golden value {name}"))?;
    assert_golden_json(name, &json)
}

fn assert_golden_json(name: &str, json: &str) -> Result<()> {
    let dir = golden_dir()?;
    let path = dir.join(format!("{name}.json"));
    if std::env::var_os("GOLDEN_UPDATE").is_some() {
        fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        fs::write(&path, json.as_bytes())
            .with_context(|| format!("writing golden {}", path.display()))?;
        return Ok(());
    }
    let expected = fs::read_to_string(&path).with_context(|| {
        format!(
            "missing golden {} — seed it once with GOLDEN_UPDATE=1 and review the diff",
            path.display()
        )
    })?;
    let expected_value: serde_json::Value = serde_json::from_str(&expected)
        .with_context(|| format!("parsing expected source facts for {name}"))?;
    let actual_value: serde_json::Value = serde_json::from_str(json)
        .with_context(|| format!("parsing actual source facts for {name}"))?;
    if expected_value != actual_value {
        let expected_lines = expected.lines().count();
        let actual_lines = json.lines().count();
        let first_diff = expected
            .lines()
            .zip(json.lines())
            .position(|(left, right)| left != right);
        bail!(
            "golden mismatch for {name}: expected {expected_lines} lines, got {actual_lines}, \
             first difference at line {}",
            first_diff.map_or(0, |index| index.saturating_add(1))
        );
    }
    Ok(())
}

pub fn digest<T: serde::Serialize>(value: &T) -> Result<String> {
    census_domain::model::serialized_digest(value)
        .map_err(|error| anyhow::anyhow!("serializing a value for its digest: {error}"))
}
