//! The `merge-coaches` subcommand: merge and validate coach-lane fragments into one importable CSV.
mod judge;
mod load;
mod report;
use anyhow::{bail, Context, Result};
use clap::Args;
use load::load_rows;
use regex::Regex;
use report::print_report;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

const HEADER: &[&str] = &[
    "school",
    "city",
    "state",
    "sport",
    "role",
    "coach_name",
    "public_professional_email",
    "ad_name",
    "ad_email",
    "source_url",
    "last_observed",
];

const PERSONAL_MAIL: &[&str] = &[
    "gmail.com",
    "yahoo.com",
    "hotmail.com",
    "outlook.com",
    "aol.com",
    "icloud.com",
    "me.com",
    "live.com",
    "msn.com",
    "comcast.net",
    "sbcglobal.net",
    "att.net",
    "verizon.net",
    "protonmail.com",
    "proton.me",
    "ymail.com",
    "mail.com",
    "aim.com",
    "earthlink.net",
    "juno.com",
    "rr.com",
    "cox.net",
    "windstream.net",
    "centurytel.net",
    "frontier.com",
];
static URL_RE: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"^https?://[^\s]+$").ok());
static URL_FIND: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"https?://\S+").ok());
static PHONE_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(\+?\d[\d\s().-]{7,}\d)").ok());
static EMAIL_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}$").ok());
static VACANT_RE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?i)^(vacant|tba|tbd|n/?a|none|unknown|-+)$").ok());

// Compiled once per process with no panic path: a malformed pattern yields `None`, and every check
// treats an unavailable pattern as "nothing matched" rather than rejecting the corpus.
fn url_re() -> Option<&'static Regex> {
    URL_RE.as_ref()
}
fn url_find() -> Option<&'static Regex> {
    URL_FIND.as_ref()
}
fn phone_re() -> Option<&'static Regex> {
    PHONE_RE.as_ref()
}
fn email_re() -> Option<&'static Regex> {
    EMAIL_RE.as_ref()
}
fn vacant_re() -> Option<&'static Regex> {
    VACANT_RE.as_ref()
}

#[derive(Debug, Clone)]
pub(super) struct Row {
    school: String,
    city: String,
    state: String,
    sport: String,
    role: String,
    coach_name: String,
    public_professional_email: String,
    ad_name: String,
    ad_email: String,
    source_url: String,
    last_observed: String,
}

impl Row {
    pub(super) fn from_fields(fields: Vec<String>) -> Self {
        load::from_fields(fields)
    }
    pub(super) fn to_fields(&self) -> Vec<String> {
        load::to_fields(self)
    }
    pub(super) fn dedupe_key(&self) -> (String, String, String, String) {
        load::dedupe_key(self)
    }
    pub(super) fn normalize(&mut self) {
        load::normalize(self)
    }
    pub(super) fn judge(&self, state: &str) -> Option<String> {
        judge::judge(self, state)
    }
}

#[derive(Debug, Args)]
pub(super) struct MergeCoachesArgs {
    #[arg(long, value_name = "DIR")]
    pub(super) fragments: PathBuf,
    #[arg(long, value_name = "CSV")]
    pub(super) out: PathBuf,
    #[arg(long, value_name = "MD")]
    pub(super) report: PathBuf,
}

#[derive(Debug, Default)]
struct StateCounts {
    rows: usize,
    kept: usize,
    duplicates: usize,
    rejected: usize,
}

#[derive(Debug, Clone)]
struct Rejection {
    state: String,
    line_no: usize,
    reason: String,
    school: String,
    coach_name: String,
    role: String,
    source_url: String,
}

pub(super) fn run_merge_coaches(args: &MergeCoachesArgs) -> Result<()> {
    if !args.fragments.is_dir() {
        bail!("no fragment directory {}", args.fragments.display());
    }
    let csv_files = collect_csv_files(&args.fragments)?;
    let pass = process_files(&csv_files)?;
    write_merged_csv(&pass.kept, args)?;
    print_report(
        &pass.kept,
        &pass.per_state,
        &pass.rejects,
        &pass.broken,
        &args.out,
        &args.report,
    );
    Ok(())
}

/// What one pass over the fragment files produced.
#[derive(Default)]
struct MergePass {
    kept: KeptRows,
    per_state: BTreeMap<String, StateCounts>,
    rejects: Vec<Rejection>,
    broken: Vec<(String, String)>,
}

fn process_files(csv_files: &[PathBuf]) -> Result<MergePass> {
    let mut pass = MergePass::default();
    for path in csv_files {
        let state = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_uppercase();
        let (rows, problem) = load_rows(path, &state);
        if let Some(problem) = problem {
            pass.broken.push((
                path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                problem,
            ));
            continue;
        }
        for (line_no, mut row) in rows {
            let counts = pass.per_state.entry(state.clone()).or_default();
            counts.rows = counts.rows.saturating_add(1);
            row.normalize();
            if let Some(reason) = Row::judge(&row, &state) {
                counts.rejected = counts.rejected.saturating_add(1);
                pass.rejects.push(Rejection {
                    state: state.clone(),
                    line_no,
                    reason,
                    school: row.school.clone(),
                    coach_name: row.coach_name.clone(),
                    role: row.role.clone(),
                    source_url: row.source_url.clone(),
                });
                continue;
            }
            let key = row.dedupe_key();
            if !pass.kept.contains_key(&key) {
                pass.kept.insert(key, row);
                counts.kept = counts.kept.saturating_add(1);
            } else {
                counts.duplicates = counts.duplicates.saturating_add(1);
                if let Some(existing) = pass.kept.get(&key) {
                    if pick_richer(&row, existing) {
                        pass.kept.insert(key, row);
                    }
                }
            }
        }
    }
    Ok(pass)
}

/// The merge key: lowercased school, state, lowercased sport, lowercased role.
type RowKey = (String, String, String, String);

/// Merged rows by their dedupe key.
type KeptRows = BTreeMap<RowKey, Row>;

fn collect_csv_files(frag_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut csv_files: Vec<PathBuf> = Vec::new();
    for entry in frag_dir
        .read_dir()
        .with_context(|| "reading fragment directory")?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "csv") {
            if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                if stem.len() == 2 && stem.is_ascii() {
                    csv_files.push(path);
                }
            }
        }
    }
    csv_files.sort();
    Ok(csv_files)
}

fn pick_richer(new: &Row, existing: &Row) -> bool {
    let new_has_email = !new.public_professional_email.trim().is_empty();
    let old_has_email = !existing.public_professional_email.trim().is_empty();
    (new_has_email && !old_has_email) || new.last_observed.trim() > existing.last_observed.trim()
}

fn write_merged_csv(
    kept: &BTreeMap<(String, String, String, String), Row>,
    args: &MergeCoachesArgs,
) -> Result<()> {
    let mut writer = csv::Writer::from_path(&args.out).with_context(|| "opening output CSV")?;
    writer
        .write_record(HEADER)
        .with_context(|| "writing CSV header")?;
    for row in kept.values() {
        writer
            .write_record(row.to_fields())
            .with_context(|| "writing CSV row")?;
    }
    writer.flush().with_context(|| "flushing CSV")?;
    Ok(())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
