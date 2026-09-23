//! `replay`: the deterministic offline parser replay `AGENTS.md` documents.
//!
//! For one source, read every capture committed under
//! `crates/midwest-census/tests/fixtures/<name>/` and run each body through the same public parse
//! surface the source's fixture tests drive, printing what the parser published: entity counts,
//! route counts and the published names. Nothing is fetched, no clock is read, no store is opened
//! and no environment is consulted, so the same tree prints the same bytes on every run - which is
//! what makes this the verb to reach for while `midwest-serve` holds the store, on a machine with
//! no network, or when a capture has to be re-read without re-crawling its host.
//!
//! The arms live in [`cases`]: one function per source, each selecting the parse entry point a
//! capture's own name and body describe. No arm re-implements a parser, re-loads a fixture or
//! compares against a golden file - `tests/parity_*.rs` own the comparisons and `tests/golden/` the
//! records. A record is read only for an input a capture cannot state about itself (a WIAA result
//! file's archive year), so `xtask` keeps no second copy of the harnesses' tables.
//!
//! Absent directories, an empty directory, a capture no arm claims, an unreadable fixture record and
//! a body whose parser refuses it are all errors: this command never reports a missing capture as an
//! empty parse.

mod cases;

use crate::paths;
use crate::source_fixture;
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Every capture body of one fixture directory, keyed by file name.
pub(super) type Captures = BTreeMap<String, String>;

/// Replay one source's committed fixture captures, in file-name order.
pub fn run(name: &str) -> Result<()> {
    let dir = paths::fixtures_dir().join(name);
    if !dir.is_dir() {
        return source_fixture::absent(name, &dir);
    }
    let corpus = read_captures(&dir)?;
    let golden = golden_dir();
    if corpus.is_empty() {
        bail!(
            "no captures under {}: the directory holds no body to replay",
            paths::relative(&dir)
        );
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

/// One capture of a fixture directory, with everything an arm reads it against.
pub(super) struct Capture<'a> {
    /// The source whose fixture directory holds the capture.
    source: &'a str,
    /// The capture's file name, as committed.
    file: &'a str,
    /// The capture's bytes.
    body: &'a str,
    /// Every capture of the same directory, by file name: a roster resolves its team through its
    /// site's index, a `/raw` body derives its URL from the meet's results page.
    corpus: &'a Captures,
    /// `crates/midwest-census/tests/golden/`, where a fixture record carries the inputs its capture
    /// cannot state (a WIAA result file's archive year).
    golden: &'a Path,
}

impl Capture<'_> {
    /// The value `field` of this capture's fixture record under `tests/golden/` (`<source>__<file>.json`).
    ///
    /// A record holds what a body cannot say about itself: a result file publishes no season, so the
    /// lane that captured it recorded the archive year beside the expected parse. Replay reads that
    /// one field rather than keeping a second copy of the harness's tables in `xtask`.
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

/// A file name no arm of `source` has a parse path for: refused with its name rather than skipped,
/// so a newly committed capture fails this verb until the parser that reads it is named here.
fn unmapped(source: &str, file: &str) -> Result<String> {
    bail!("no `{source}` replay path for `{file}`: this capture's parser is not selected yet")
}

/// The parity harnesses' own rule for a total reader: a body that yields nothing did not parse.
///
/// Every arm applies it where an empty result would mean the body was not the page or payload it
/// claims to be; the captures whose expected parse *is* empty are named in their arms instead.
fn ensure_rows(file: &str, rows: usize, what: &str) -> Result<()> {
    if rows == 0 {
        bail!("{file} yielded no {what}: the body did not parse");
    }
    Ok(())
}

/// Every capture body of one fixture directory, keyed by file name.
///
/// A few arms read a capture against a sibling page, so the walk reads the whole directory and hands
/// each arm the corpus rather than one body.
fn read_captures(dir: &Path) -> Result<Captures> {
    let mut captures = Captures::new();
    for path in source_fixture::files_under(dir)? {
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

/// Where the fixture records live: beside the fixture directories, under the crate's `tests/`.
fn golden_dir() -> PathBuf {
    let fixtures = paths::fixtures_dir();
    let tests = fixtures.parent().unwrap_or(&fixtures);
    tests.join(GOLDEN_DIR)
}

/// The fixture contract's prose file: every fixture directory documents itself with one (§64).
const FIXTURE_CONTRACT: &str = "README.md";

/// The directory of expected-parse records the parity harnesses compare against.
const GOLDEN_DIR: &str = "golden";
