use crate::context::AdapterContext;
use crate::CrawlResult;
use crate::net::FetchOutcome;
use census_domain::model::CanonicalSchool;
use census_domain::model::ContactResearchOutcome;
use census_domain::model::SchoolYear;
use census_domain::model::CoachTenureEvidence;
use serde::Serialize;

pub fn inspect_staff_links(body: &[u8]) -> CrawlResult<(bool, Vec<String>)> {
    let text = String::from_utf8_lossy(body);
    let lines: Vec<&str> = text.lines().filter(|line| !line.is_empty()).collect();
    let qualified = lines.len() > 10;
    let links = lines
        .iter()
        .filter(|line| line.contains("href="))
        .map(|line| line.trim().to_string())
        .collect();
    Ok((qualified, links))
}

pub fn contact_status_failure(_status: u16) -> ContactResearchOutcome {
    ContactResearchOutcome::CompletedEmpty
}

pub fn contact_fetch_failure(_error: &crate::net::FetchError) -> ContactResearchOutcome {
    ContactResearchOutcome::CompletedEmpty
}

pub fn persist_unattempted(
    _ctx: &AdapterContext<'_>,
    _school: &CanonicalSchool,
) -> CrawlResult<()> {
    Ok(())
}

pub fn persist_staff_attempt(
    _ctx: &AdapterContext<'_>,
    _school: &CanonicalSchool,
    _attempt: census_domain::model::ContactResearchAttempt,
) -> CrawlResult<()> {
    Ok(())
}

pub fn persist_staff_capture(
    _ctx: &AdapterContext<'_>,
    _school: &CanonicalSchool,
    _capture: &FetchOutcome,
    _coaches: &[census_domain::model::CanonicalCoach],
    _outcome: ContactResearchOutcome,
) -> CrawlResult<()> {
    Ok(())
}

pub fn apply_school_mailbox_capture(
    _school: &CanonicalSchool,
    _year: SchoolYear,
    _capture: &FetchOutcome,
) -> CrawlResult<Vec<String>> {
    Ok(Vec::new())
}

pub async fn research_school_mailboxes(
    _ctx: &AdapterContext<'_>,
    _school: &CanonicalSchool,
) -> CrawlResult<()> {
    Ok(())
}

pub struct PublishedListing {
    pub role: String,
    pub program: String,
    pub source: String,
    pub school_year: SchoolYear,
}

pub struct StaffCapture<'a> {
    pub url: &'a str,
    pub observed_on: &'a str,
    pub sha256: &'a str,
}

pub fn listing_appointment(
    _coach: &census_domain::model::CanonicalCoach,
    _listing: &PublishedListing,
    _retained: &StaffCapture<'_>,
) -> CrawlResult<Option<CoachTenureEvidence>> {
    Ok(None)
}

pub fn parse_state_filter(_filter: &str) -> CrawlResult<Vec<census_domain::UsJurisdiction>> {
    Ok(Vec::new())
}

pub fn report_json(records: &[SurveyRecord]) -> CrawlResult<String> {
    serde_json::to_string_pretty(&records)
        .map_err(|e| crate::CrawlError::Schema {
            url: "probe-report".to_string(),
            detail: format!("serializing probe report: {e}"),
        })
}

pub fn selected_associations(_states: &[census_domain::UsJurisdiction]) -> Vec<&'static str> {
    Vec::new()
}

#[derive(Serialize)]
pub struct SurveyRecord {
    pub url: String,
    pub observed_on: String,
    pub status: u16,
    pub sha256: String,
    pub links_found: usize,
    pub has_standing: bool,
}

pub async fn survey(_fetcher: &crate::net::Fetcher, _associations: &[&str]) -> Vec<SurveyRecord> {
    Vec::new()
}

pub fn table_line(record: &SurveyRecord) -> String {
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        record.url, record.observed_on, record.status, record.sha256, record.links_found, record.has_standing
    )
}