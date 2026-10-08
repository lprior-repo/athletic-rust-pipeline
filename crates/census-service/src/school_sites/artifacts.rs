use super::queue::QueueStats;
use anyhow::{Context, Result};
use serde::Serialize;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct Report {
    pub queue: QueueStats,
    pub out_dir: String,
    pub planned: usize,
    pub crawled: usize,
    pub empty: usize,
    pub skipped: usize,
    pub failed: usize,
    pub emails: usize,
    pub coach_contacts: usize,
    pub ad_contacts: usize,
    pub requests: usize,
    pub errors: usize,
    pub failures: Vec<Failure>,
}

#[derive(Debug, Serialize)]
pub struct Failure {
    pub state: String,
    pub school: String,
    pub website: String,
    pub detail: String,
}

pub(super) fn write_report(path: &Path, report: &Report) -> Result<()> {
    let file =
        std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?;
    let mut writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, report)
        .context("encoding the school-sites report")?;
    writer
        .flush()
        .with_context(|| format!("writing {}", path.display()))
}
