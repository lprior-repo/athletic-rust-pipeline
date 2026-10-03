mod cases;

use crate::paths;
use crate::source_fixture;
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub(super) type Captures = BTreeMap<String, String>;

pub fn run(name: &str) -> Result<()> {
    let dir = paths::fixtures_dir().join(name);
    if !dir.is_dir() {
        return source_fixture::absent(name, &dir);
    }
    let corpus = read_captures(name, &dir)?;
    let golden = golden_dir();
    if corpus.is_empty() {
        bail!(
            "no captures under {}: the directory holds no body to replay",
            paths::relative(&dir)
        );
    }
    if name == "coach_directories" {
        println!("qualification: five root responses; metadata and the separate survey corpus are not response inputs");
    }
    println!(
        "source {name}: {} capture(s) under {}, offline, no store, no clock",
        corpus.len(),
        paths::relative(&dir)
    );
    for (file, body) in &corpus {
        let capture = Capture {
            source: name,
            file,
            body,
            corpus: &corpus,
            golden: &golden,
        };
        let summary = cases::replay(&capture)
            .with_context(|| format!("replaying {}", paths::relative(&dir.join(file))))?;
        println!("{file:<48}  {summary}");
    }
    println!();
    println!("{} capture(s) replayed for source {name}", corpus.len());
    Ok(())
}

pub(super) struct Capture<'a> {
    source: &'a str,
    file: &'a str,
    body: &'a str,
    corpus: &'a Captures,
    golden: &'a Path,
}

impl Capture<'_> {
    fn recorded(&self, field: &str) -> Result<String> {
        let name = format!("{}__{}.json", self.source, self.file);
        let path = self.golden.join(&name);
        let body = fs::read_to_string(&path)
            .with_context(|| format!("reading the fixture record {}", paths::relative(&path)))?;
        let record: serde_json::Value = serde_json::from_str(&body)
            .with_context(|| format!("decoding the fixture record {name}"))?;
        match record.get(field) {
            Some(serde_json::Value::String(value)) => Ok(value.clone()),
            Some(serde_json::Value::Number(value)) => Ok(value.to_string()),
            _ => bail!("the fixture record {name} carries no `{field}`"),
        }
    }
}

fn unmapped(source: &str, file: &str) -> Result<String> {
    bail!("no `{source}` replay path for `{file}`: this capture's parser is not selected yet")
}

fn ensure_rows(file: &str, rows: usize, what: &str) -> Result<()> {
    if rows == 0 {
        bail!("{file} yielded no {what}: the body did not parse");
    }
    Ok(())
}

fn read_captures(source: &str, dir: &Path) -> Result<Captures> {
    let mut captures = Captures::new();
    for path in cases::capture_paths(source, dir)? {
        let name = path
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        if name.is_empty() || name == FIXTURE_CONTRACT {
            continue;
        }
        let body = fs::read_to_string(&path)
            .with_context(|| format!("reading {}", paths::relative(&path)))?;
        captures.insert(name, body);
    }
    Ok(captures)
}

fn golden_dir() -> PathBuf {
    let fixtures = paths::fixtures_dir();
    let tests = fixtures
        .parent()
        .map_or(fixtures.as_path(), core::convert::identity);
    tests.join(GOLDEN_DIR)
}

const FIXTURE_CONTRACT: &str = "README.md";

const GOLDEN_DIR: &str = "golden";
