use crate::ai_cache::fingerprint;
use crate::model::{MatchRecord, Prospect};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Write};
use std::path::Path;

const SCHEMA_VERSION: u32 = 3;
const VERSION_STRING: &str = "exhaustive-v5-deterministic-first";

#[derive(Debug, Serialize, Deserialize)]
pub struct RunManifest {
    pub fingerprint: String,
    pub schema_version: u32,
    pub scope: String,
    pub input_display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    pub fingerprint: String,
    pub total_rows: u64,
    pub searchable_rows: u64,
    pub missing_name_rows: u64,
    pub completed_rows: u64,
    pub pending_rows: u64,
    pub retryable_rows: u64,
    pub status_counts: HashMap<String, u64>,
    pub logical_minimum_searches: u64,
    pub complete: bool,
    pub sheets: Vec<String>,
}

struct Sha256Writer {
    hasher: Sha256,
}

impl Sha256Writer {
    fn new() -> Self {
        Self {
            hasher: Sha256::new(),
        }
    }
}

impl Write for Sha256Writer {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.hasher.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn bind_run(input: &Path, config_path: &Path, out_dir: &Path, scope: &str) -> Result<String> {
    if scope.trim().is_empty() {
        anyhow::bail!("run scope must not be empty");
    }
    let input_file = File::open(input).context("opening input workbook")?;
    let mut sha_writer = Sha256Writer::new();
    let mut buffered_reader = BufReader::new(input_file);
    io::copy(&mut buffered_reader, &mut sha_writer).context("streaming input workbook")?;
    let input_digest = format!("{:x}", sha_writer.hasher.finalize());

    let config_bytes = fs::read(config_path).context("reading config")?;
    let config_digest = format!("{:x}", Sha256::digest(&config_bytes));
    let fingerprint = fingerprint(&[
        &input_digest,
        &config_digest,
        scope,
        VERSION_STRING,
        &crate::extract::AI_SCHEMA_VERSION.to_string(),
    ]);
    let input_display = input.display().to_string();
    let manifest = RunManifest {
        fingerprint: fingerprint.clone(),
        schema_version: SCHEMA_VERSION,
        scope: scope.to_owned(),
        input_display,
    };

    let manifest_path = out_dir.join("run-manifest.json");
    if manifest_path.exists() {
        let existing_content =
            fs::read_to_string(&manifest_path).context("reading existing manifest")?;
        let existing_manifest: RunManifest =
            serde_json::from_str(&existing_content).context("parsing existing manifest")?;
        if existing_manifest.fingerprint != fingerprint {
            anyhow::bail!(
                "Fingerprint mismatch. Existing manifest has fingerprint {}, expected {}. Recommend using a new output directory.",
                existing_manifest.fingerprint,
                fingerprint
            );
        }
        if existing_manifest.schema_version != SCHEMA_VERSION {
            anyhow::bail!(
                "run manifest schema mismatch: found {}, expected {}",
                existing_manifest.schema_version,
                SCHEMA_VERSION
            );
        }
        if existing_manifest.scope != scope
            || existing_manifest.input_display != manifest.input_display
        {
            anyhow::bail!("run manifest metadata does not match the requested input and scope");
        }
    }

    let artifacts = [
        "checkpoint.jsonl",
        "search-cache.jsonl",
        "ai-cache.jsonl",
        "coverage.json",
        "matches.jsonl",
        "matches.csv",
        "unresolved.csv",
        "run-manifest.json.tmp",
        "coverage.json.tmp",
    ];
    if !manifest_path.exists()
        && artifacts
            .iter()
            .map(|name| out_dir.join(name))
            .any(|path| path.exists())
    {
        anyhow::bail!(
            "Refusing to bind. Existing run artifacts found without a matching run-manifest.json. \
             Please use a new output directory."
        );
    }

    let temp_manifest_path = out_dir.join("run-manifest.json.tmp");
    {
        let file = File::create(&temp_manifest_path).context("creating temp manifest")?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &manifest).context("serializing manifest")?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
    }
    fs::rename(&temp_manifest_path, &manifest_path).context("renaming manifest")?;
    File::open(out_dir)?.sync_all()?;

    Ok(fingerprint)
}

pub fn is_final(key: &str, records: &HashMap<String, MatchRecord>) -> bool {
    records.get(key).is_some_and(terminal_record)
}

fn terminal_record(record: &MatchRecord) -> bool {
    let missing_name =
        record.prospect.first_name.trim().is_empty() && record.prospect.last_name.trim().is_empty();
    if missing_name {
        return record.status == "INPUT_ERROR";
    }
    matches!(
        record.status.as_str(),
        "MATCH" | "CLOSE_MATCH" | "REVIEW" | "NO_MATCH"
    )
}

pub fn write_coverage(
    out_dir: &Path,
    fingerprint: &str,
    prospects: &[Prospect],
    records: &HashMap<String, MatchRecord>,
) -> Result<()> {
    write_report(out_dir, &build_report(fingerprint, prospects, records)?)
}

pub fn build_report(
    fingerprint: &str,
    prospects: &[Prospect],
    records: &HashMap<String, MatchRecord>,
) -> Result<CoverageReport> {
    let population = population_index(prospects)?;
    records.iter().try_for_each(|(key, record)| {
        let Some((sheet, excel_row)) = population.get(key.as_str()) else {
            anyhow::bail!("orphan checkpoint record for source key {key}");
        };
        if record.source_key.as_str() != key.as_str()
            || record.prospect.source_key.as_str() != key.as_str()
            || record.prospect.sheet.as_str() != *sheet
            || record.prospect.excel_row != *excel_row
        {
            anyhow::bail!("checkpoint record metadata mismatch for source key {key}");
        }
        Ok(())
    })?;

    let total_rows =
        u64::try_from(prospects.len()).context("converting prospects length to u64")?;
    let (
        searchable_rows,
        missing_name_rows,
        completed_rows,
        pending_rows,
        retryable_rows,
        status_counts,
        sheets,
    ) = prospects.iter().fold(
        (
            0_u64,
            0_u64,
            0_u64,
            0_u64,
            0_u64,
            HashMap::<String, u64>::new(),
            Vec::new(),
        ),
        |(searchable, missing, completed, pending, retryable, mut status_counts, mut sheets),
         prospect| {
            let has_name =
                !prospect.first_name.trim().is_empty() || !prospect.last_name.trim().is_empty();
            let (searchable, missing) = if has_name {
                (searchable.saturating_add(1), missing)
            } else {
                (searchable, missing.saturating_add(1))
            };

            let (completed, pending, retryable) = match records.get(&prospect.source_key) {
                Some(record) => {
                    let count = status_counts.entry(record.status.clone()).or_insert(0);
                    *count = count.saturating_add(1);
                    if is_final(&prospect.source_key, records) {
                        (completed.saturating_add(1), pending, retryable)
                    } else {
                        (completed, pending, retryable.saturating_add(1))
                    }
                }
                None => (completed, pending.saturating_add(1), retryable),
            };

            if !sheets.contains(&prospect.sheet) {
                sheets.push(prospect.sheet.clone());
            }
            (
                searchable,
                missing,
                completed,
                pending,
                retryable,
                status_counts,
                sheets,
            )
        },
    );

    let logical_minimum_searches = searchable_rows.saturating_mul(2);
    let complete =
        total_rows > 0 && completed_rows == total_rows && pending_rows == 0 && retryable_rows == 0;

    Ok(CoverageReport {
        fingerprint: fingerprint.to_owned(),
        total_rows,
        searchable_rows,
        missing_name_rows,
        completed_rows,
        pending_rows,
        retryable_rows,
        status_counts,
        logical_minimum_searches,
        complete,
        sheets,
    })
}

pub fn write_report(out_dir: &Path, report: &CoverageReport) -> Result<()> {
    validate_coverage_binding(out_dir, &report.fingerprint)?;

    let temp_path = out_dir.join("coverage.json.tmp");
    {
        let file = File::create(&temp_path).context("creating temp coverage file")?;
        let mut writer = BufWriter::new(file);
        serde_json::to_writer(&mut writer, &report).context("serializing coverage report")?;
        writer.write_all(b"\n")?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
    }
    fs::rename(&temp_path, out_dir.join("coverage.json")).context("renaming coverage file")?;
    File::open(out_dir)?.sync_all()?;
    Ok(())
}

fn population_index(prospects: &[Prospect]) -> Result<HashMap<&str, (&str, u32)>> {
    prospects
        .iter()
        .try_fold(HashMap::new(), |mut index, prospect| {
            if prospect.source_key.trim().is_empty() {
                anyhow::bail!("population contains an empty source key");
            }
            if index
                .insert(
                    prospect.source_key.as_str(),
                    (prospect.sheet.as_str(), prospect.excel_row),
                )
                .is_some()
            {
                anyhow::bail!(
                    "population contains duplicate source key {}",
                    prospect.source_key
                );
            }
            Ok(index)
        })
}

fn validate_coverage_binding(out_dir: &Path, fingerprint: &str) -> Result<()> {
    let manifest_path = out_dir.join("run-manifest.json");
    if manifest_path.exists() {
        let content = fs::read_to_string(&manifest_path).context("reading run manifest")?;
        let manifest: RunManifest =
            serde_json::from_str(&content).context("parsing run manifest")?;
        if manifest.schema_version != SCHEMA_VERSION {
            anyhow::bail!("coverage run manifest schema mismatch");
        }
        if manifest.fingerprint.trim().is_empty()
            || manifest.scope.trim().is_empty()
            || manifest.input_display.trim().is_empty()
        {
            anyhow::bail!("coverage run manifest metadata is incomplete");
        }
        if manifest.fingerprint != fingerprint {
            anyhow::bail!("coverage fingerprint does not match the run manifest");
        }
    }
    let coverage_path = out_dir.join("coverage.json");
    if coverage_path.exists() {
        let content = fs::read_to_string(&coverage_path).context("reading existing coverage")?;
        let existing: CoverageReport =
            serde_json::from_str(&content).context("parsing existing coverage")?;
        if existing.fingerprint != fingerprint {
            anyhow::bail!("coverage fingerprint mismatch");
        }
    }
    Ok(())
}

impl CoverageReport {
    pub fn record_committed(
        &mut self,
        previous: Option<&MatchRecord>,
        record: &MatchRecord,
    ) -> Result<()> {
        let mut next = self.clone();
        next.remove_previous(previous)?;
        increment(next.status_counts.entry(record.status.clone()).or_insert(0))?;
        if terminal_record(record) {
            increment(&mut next.completed_rows)?;
        } else {
            increment(&mut next.retryable_rows)?;
        }
        next.complete = next.total_rows > 0
            && next.completed_rows == next.total_rows
            && next.pending_rows == 0
            && next.retryable_rows == 0;
        *self = next;
        Ok(())
    }

    fn remove_previous(&mut self, previous: Option<&MatchRecord>) -> Result<()> {
        let Some(previous) = previous else {
            return decrement(&mut self.pending_rows);
        };
        let count = self
            .status_counts
            .get_mut(&previous.status)
            .context("previous checkpoint status absent from coverage")?;
        decrement(count)?;
        if *count == 0 {
            self.status_counts.remove(&previous.status);
        }
        if terminal_record(previous) {
            decrement(&mut self.completed_rows)
        } else {
            decrement(&mut self.retryable_rows)
        }
    }
}

fn increment(value: &mut u64) -> Result<()> {
    *value = value.checked_add(1).context("coverage counter overflow")?;
    Ok(())
}

fn decrement(value: &mut u64) -> Result<()> {
    *value = value.checked_sub(1).context("coverage counter underflow")?;
    Ok(())
}

#[cfg(test)]
#[path = "coverage_tests.rs"]
mod tests;
