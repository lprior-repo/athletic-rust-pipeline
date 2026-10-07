use super::contacts::LaneRow;
use super::queue::{PlannedSite, QueueStats};
use anyhow::{Context, Result};
use census_crawl::school_sites::{PageEvidence, SiteOutcome};
use census_domain::model::{CONTACT_COLUMNS, CONTACT_PROOF_COLUMN};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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

fn lane_header() -> impl Iterator<Item = &'static str> {
    CONTACT_COLUMNS
        .iter()
        .copied()
        .chain([CONTACT_PROOF_COLUMN])
}

fn write_rows(path: &Path, rows: &[LaneRow]) -> Result<()> {
    let mut writer =
        csv::Writer::from_path(path).with_context(|| format!("creating {}", path.display()))?;
    writer
        .write_record(lane_header())
        .with_context(|| format!("writing the contact-lane header of {}", path.display()))?;
    for row in rows {
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
            .with_context(|| format!("writing a contact-lane row of {}", path.display()))?;
    }
    writer
        .flush()
        .with_context(|| format!("flushing {}", path.display()))
}

pub fn write_site_rows(path: &Path, rows: &[LaneRow]) -> Result<()> {
    write_rows(path, rows)
}

pub fn publish_state_fragments(dir: &Path, site_rows: &Path) -> Result<Vec<String>> {
    std::fs::create_dir_all(dir).with_context(|| format!("creating {}", dir.display()))?;
    let mut rows = read_site_rows(site_rows)?;
    rows.sort();
    rows.dedup();
    let mut by_state: BTreeMap<&str, Vec<&LaneRow>> = BTreeMap::new();
    for row in &rows {
        by_state.entry(row.state.as_str()).or_default().push(row);
    }
    let mut written = Vec::new();
    for (state, state_rows) in by_state {
        let path = dir.join(format!("{state}.csv"));
        let owned: Vec<LaneRow> = state_rows.into_iter().cloned().collect();
        write_rows(&path, &owned)?;
        written.push(path.display().to_string());
    }
    Ok(written)
}

fn read_site_rows(dir: &Path) -> Result<Vec<LaneRow>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error).with_context(|| format!("reading site rows {}", dir.display()))
        }
    };
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let path = entry
            .with_context(|| format!("reading site rows {}", dir.display()))?
            .path();
        if path.extension().is_some_and(|ext| ext == "csv") {
            paths.push(path);
        }
    }
    paths.sort();
    let mut rows = Vec::new();
    for path in paths {
        rows.extend(read_rows(&path)?);
    }
    Ok(rows)
}

fn read_rows(path: &Path) -> Result<Vec<LaneRow>> {
    let mut reader =
        csv::Reader::from_path(path).with_context(|| format!("reading {}", path.display()))?;
    let header = reader
        .headers()
        .with_context(|| format!("reading the header of {}", path.display()))?;
    if !lane_header().eq(header.iter()) {
        return Err(anyhow::anyhow!(
            "{} does not carry the twelve-column contact-lane header",
            path.display()
        ));
    }
    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.with_context(|| format!("parsing {}", path.display()))?;
        let mut values = [""; 12];
        for (index, slot) in values.iter_mut().enumerate() {
            *slot = match record.get(index) {
                Some(value) => value,
                None => {
                    return Err(anyhow::anyhow!(
                        "{} has a row with {} of 12 contact-lane columns",
                        path.display(),
                        record.len()
                    ))
                }
            };
        }
        let [school, city, state, sport, role, coach_name, public_professional_email, ad_name, ad_email, source_url, last_observed, verified_proof_digest] =
            values;
        rows.push(LaneRow {
            school: school.to_string(),
            city: city.to_string(),
            state: state.to_string(),
            sport: sport.to_string(),
            role: role.to_string(),
            coach_name: coach_name.to_string(),
            public_professional_email: public_professional_email.to_string(),
            ad_name: ad_name.to_string(),
            ad_email: ad_email.to_string(),
            source_url: source_url.to_string(),
            last_observed: last_observed.to_string(),
            verified_proof_digest: verified_proof_digest.to_string(),
        });
    }
    Ok(rows)
}

pub fn write_report(path: &Path, report: &Report) -> Result<()> {
    let body = serde_json::to_string_pretty(report).context("encoding the school-sites report")?;
    std::fs::write(path, body).with_context(|| format!("writing {}", path.display()))
}
