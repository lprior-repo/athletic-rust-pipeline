use super::queue::QueueStats;
use super::research::{CoachResearchStatus, ResearchStatus, SchoolMailboxCandidate};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Serialize, Deserialize)]
pub(super) struct SiteRecord {
    pub state: String,
    pub school: String,
    pub website: String,
    pub pages: Vec<String>,
    pub page_evidence: Vec<census_crawl::school_sites::PageEvidence>,
    #[serde(default)]
    pub school_mailbox_candidates: Vec<SchoolMailboxCandidate>,
    #[serde(default)]
    pub research_status: ResearchStatus,
    #[serde(default)]
    pub coach_research_status: CoachResearchStatus,
    pub requests: usize,
    pub errors: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

pub(super) fn write_site(path: &Path, record: &SiteRecord) -> Result<()> {
    let file =
        std::fs::File::create(path).with_context(|| format!("creating {}", path.display()))?;
    let mut writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, record).context("encoding the school-site record")?;
    writer
        .flush()
        .with_context(|| format!("writing {}", path.display()))
}
