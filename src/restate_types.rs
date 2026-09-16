use crate::{
    address::AddressEvidence,
    model::{MatchRecord, Prospect},
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

pub const RESTATE_SCHEMA_VERSION: u32 = 2;
pub const MAX_ROW_ATTEMPTS: u32 = 64;
pub const MAX_INPUT_BYTES: usize = 1024 * 1024;
pub const MAX_CHECKPOINT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowInput {
    pub run_fingerprint: String,
    pub config_digest: String,
    pub prospect: Prospect,
    pub no_ai: bool,
    pub authorization_ack: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowIssue {
    pub stage: String,
    pub code: String,
    pub message: String,
    pub retryable: bool,
    pub attempt: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowDisposition {
    Complete,
    Retryable,
    OperatorRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RowOutput {
    pub record: MatchRecord,
    pub address: AddressEvidence,
    pub issues: Vec<RowIssue>,
    pub attempt: u32,
    pub disposition: RowDisposition,
}

pub fn config_digest(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).context("reading immutable Restate configuration")?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn row_key(run_fingerprint: &str, source_key: &str) -> String {
    let mut hash = Sha256::new();
    hash.update(run_fingerprint.as_bytes());
    hash.update([0]);
    hash.update(source_key.as_bytes());
    format!("{:x}", hash.finalize())
}

pub fn final_record(record: &MatchRecord) -> bool {
    matches!(
        record.status.as_str(),
        "MATCH" | "CLOSE_MATCH" | "REVIEW" | "NO_MATCH" | "INPUT_ERROR"
    )
}

pub fn validate_output(prospect: &Prospect, output: &RowOutput) -> Result<()> {
    if output.attempt == 0 || output.attempt > MAX_ROW_ATTEMPTS
        || output.record.source_key != prospect.source_key
        || serde_json::to_value(&output.record.prospect)? != serde_json::to_value(prospect)?
    {
        anyhow::bail!("Restate output immutable source metadata or attempt mismatch");
    }
    if !matches!(output.record.status.as_str(),
        "MATCH" | "CLOSE_MATCH" | "REVIEW" | "NO_MATCH" | "INPUT_ERROR" | "SEARCH_ERROR" | "AI_ERROR")
        || final_record(&output.record) != (output.disposition == RowDisposition::Complete)
        || (output.disposition == RowDisposition::Retryable && output.attempt >= MAX_ROW_ATTEMPTS)
    {
        anyhow::bail!("Restate outcome disposition contradicts row status or attempt");
    }
    let missing_name = prospect.full_name().is_empty();
    if (output.record.status == "INPUT_ERROR") != missing_name {
        anyhow::bail!("Restate input error contradicts source names");
    }
    if output.address.requires_review() && matches!(output.record.status.as_str(), "MATCH" | "CLOSE_MATCH") {
        anyhow::bail!("positive attribution contradicts address review requirement");
    }
    check_size(output, MAX_CHECKPOINT_BYTES)
}

pub fn check_size(value: &impl Serialize, limit: usize) -> Result<()> {
    serde_json::to_writer(&mut SizeLimit { bytes: 0, limit }, value)
        .context("serialized payload exceeds its byte budget or is invalid")
}

struct SizeLimit {
    bytes: usize,
    limit: usize,
}

impl std::io::Write for SizeLimit {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let size = self.bytes.checked_add(bytes.len()).filter(|size| *size <= self.limit)
            .ok_or_else(|| std::io::Error::other("payload byte limit"))?;
        self.bytes = size;
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> { Ok(()) }
}
