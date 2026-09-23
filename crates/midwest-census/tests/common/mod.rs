//! Shared support for the golden-corpus parity tests.
//!
//! A decomposition refactor must not change a single published parse result. Each parity test
//! parses a committed fixture, serializes the resulting canonical entities to stable JSON and
//! compares the bytes against the checked-in golden file under `tests/golden/`. Any refactor that
//! changes an output — ordering, a defaulted field, a normalised mark — fails the comparison.
//!
//! `GOLDEN_UPDATE=1` rewrites the golden files. It is a local seeding affordance: run the test
//! once with the variable set, inspect the diff, and commit the review. CI never sets it.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

/// `<crate>/tests`.
fn tests_dir() -> Result<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = manifest.join("tests");
    if !dir.is_dir() {
        bail!("missing test directory at {}", dir.display());
    }
    Ok(dir)
}

/// The fixture corpus: `<crawl crate>/tests/fixtures`.
///
/// The captures live with the adapters that read them, and this crate's parity tests are their
/// second consumer — resolving them through the crawl crate's manifest keeps one copy of the
/// corpus instead of a duplicate that could silently drift from the adapter's own tests.
pub fn fixtures_dir() -> Result<PathBuf> {
    Ok(crawl_crate_dir()?.join("tests").join("fixtures"))
}

/// `<repo>/crates/census-crawl`, resolved from this crate's manifest.
fn crawl_crate_dir() -> Result<PathBuf> {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let dir = manifest.join("../census-crawl");
    if !dir.is_dir() {
        bail!("missing crawl crate at {}", dir.display());
    }
    Ok(dir)
}

/// `<crate>/tests/golden`.
pub fn golden_dir() -> Result<PathBuf> {
    Ok(tests_dir()?.join("golden"))
}

/// Reads `tests/fixtures/<source>/<file>`.
pub fn fixture(source: &str, file: &str) -> Result<String> {
    let path = fixtures_dir()?.join(source).join(file);
    fs::read_to_string(&path).with_context(|| format!("reading fixture {}", path.display()))
}

/// Every regular file under `tests/fixtures/<source>`, sorted by file name.
///
/// Fixture directories hold only committed captures, so the returned list is the complete corpus
/// for that source: a parity run that skips one of them is a coverage hole.
pub fn fixtures(source: &str) -> Result<Vec<PathBuf>> {
    let dir = fixtures_dir()?.join(source);
    let entries =
        fs::read_dir(&dir).with_context(|| format!("listing fixtures in {}", dir.display()))?;
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let path = entry
            .with_context(|| format!("reading an entry of {}", dir.display()))?
            .path();
        if path.is_file() {
            paths.push(path);
        }
    }
    paths.sort();
    if paths.is_empty() {
        bail!("no fixtures under {}", dir.display());
    }
    Ok(paths)
}

/// The fixture file name (last path component) as a `String`.
pub fn file_name(path: &Path) -> Result<String> {
    let name = path.file_name().context("fixture path has no file name")?;
    Ok(name.to_string_lossy().into_owned())
}

/// Compares `value`'s pretty JSON with `tests/golden/<name>.json`.
///
/// The comparison is on bytes, so field order, number formatting and collection order all count.
pub fn assert_golden<T: serde::Serialize>(name: &str, value: &T) -> Result<()> {
    let json = serde_json::to_string_pretty(value)
        .with_context(|| format!("serializing golden value {name}"))?;
    assert_golden_json(name, &json)
}

/// Compares an already-serialized JSON document with `tests/golden/<name>.json`.
pub fn assert_golden_json(name: &str, json: &str) -> Result<()> {
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
    if expected != json {
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

/// A stable one-line digest of a serializable value, for counting assertions in a parity test.
pub fn digest<T: serde::Serialize>(value: &T) -> Result<String> {
    use sha2::{Digest, Sha256};

    let json = serde_json::to_string(value).context("serializing a value for its digest")?;
    let hash = Sha256::digest(json.as_bytes());
    let mut out = String::with_capacity(hash.len().saturating_mul(2));
    for byte in hash {
        out.push_str(&format!("{byte:02x}"));
    }
    Ok(out)
}
