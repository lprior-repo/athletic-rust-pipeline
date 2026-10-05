use super::contacts::LaneRow;
use super::queue::{PlannedSite, QueueStats};
use anyhow::{Context, Result};
use census_crawl::school_sites::{PageEvidence, SiteOutcome};
use census_domain::model::{CONTACT_COLUMNS, CONTACT_PROOF_COLUMN};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Serialize)]
pub struct Report {
    pub queue: QueueStats,
    pub out_dir: String,
    pub fragments: String,
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

#[derive(Debug, Serialize)]
pub struct SiteRecord<'a> {
    state: &'a str,
    school: &'a str,
    website: &'a str,
    emails: &'a [String],
    coach_hits: Vec<HitRecord<'a>>,
    ad_hits: Vec<AdRecord<'a>>,
    pages: &'a [String],
    page_evidence: &'a [PageEvidence],
    #[serde(skip_serializing_if = "Option::is_none")]
    empty: Option<bool>,
}

#[derive(Debug, Serialize)]
struct HitRecord<'a> {
    name: &'a str,
    sport: &'a str,
    context: &'a str,
    url: &'a str,
    title: &'a str,
    role: &'a str,
    gender: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    email: Option<&'a str>,
}

#[derive(Debug, Serialize)]
struct AdRecord<'a> {
    name: &'a str,
    context: &'a str,
    url: &'a str,
    title: &'a str,
}

pub fn site_record<'a>(site: &'a PlannedSite, outcome: &'a SiteOutcome) -> SiteRecord<'a> {
    SiteRecord {
        state: site.state.code(),
        school: &site.school,
        website: &site.website,
        emails: &outcome.signals.emails,
        coach_hits: outcome
            .signals
            .coach_hits
            .iter()
            .map(|hit| HitRecord {
                name: &hit.name,
                sport: hit.sport.stable_key(),
                context: &hit.context,
                url: &hit.url,
                title: &hit.title,
                role: hit.role.stable_key(),
                gender: hit.gender.stable_key(),
                email: hit.email.as_deref(),
            })
            .collect(),
        ad_hits: outcome
            .signals
            .ad_hits
            .iter()
            .map(|hit| AdRecord {
                name: &hit.name,
                context: &hit.context,
                url: &hit.url,
                title: &hit.title,
            })
            .collect(),
        pages: &outcome.signals.pages,
        page_evidence: &outcome.pages,
        empty: (!outcome.signals.has_signals()).then_some(true),
    }
}

pub fn write_site(path: &Path, record: &SiteRecord<'_>) -> Result<()> {
    let body = serde_json::to_string_pretty(record).context("encoding a school-site artifact")?;
    std::fs::write(path, body).with_context(|| format!("writing {}", path.display()))
}

pub fn write_fragments(dir: &Path, rows: &[LaneRow]) -> Result<Vec<String>> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let mut by_state: BTreeMap<&str, Vec<&LaneRow>> = BTreeMap::new();
    for row in rows {
        by_state.entry(row.state.as_str()).or_default().push(row);
    }
    let mut written = Vec::new();
    for (state, state_rows) in by_state {
        let path = dir.join(format!("{state}.csv"));
        let mut writer = csv::Writer::from_path(&path)
            .with_context(|| format!("creating {}", path.display()))?;
        writer
            .write_record(
                CONTACT_COLUMNS
                    .iter()
                    .copied()
                    .chain([CONTACT_PROOF_COLUMN]),
            )
            .context("writing the fragment header")?;
        for row in state_rows {
            writer
                .write_record([
                    row.school.as_str(),
                    row.city.as_str(),
                    row.state.as_str(),
                    row.sport.as_str(),
                    row.role.as_str(),
                    row.coach_name.as_str(),
                    row.public_professional_email.as_str(),
                    row.ad_name.as_str(),
                    row.ad_email.as_str(),
                    row.source_url.as_str(),
                    row.last_observed.as_str(),
                    row.verified_proof_digest.as_str(),
                ])
                .context("writing a fragment row")?;
        }
        writer.flush().context("flushing a fragment")?;
        written.push(path.display().to_string());
    }
    Ok(written)
}

pub fn write_report(path: &Path, report: &Report) -> Result<()> {
    let body = serde_json::to_string_pretty(report).context("encoding the school-sites report")?;
    std::fs::write(path, body).with_context(|| format!("writing {}", path.display()))
}
