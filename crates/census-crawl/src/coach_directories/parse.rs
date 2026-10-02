use crate::{CrawlError, CrawlResult};
use serde::Deserialize;
use std::collections::BTreeMap;

mod postal;

const MAX_SOURCE_ROWS: usize = 20_000;

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryPage {
    #[serde(default)]
    pub current_page: usize,
    #[serde(default)]
    pub total_pages: usize,
    #[serde(default)]
    pub total_results: usize,
    #[serde(default)]
    pub results: Vec<DirectorySchool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(from = "postal::DirectoryWire")]
pub struct DirectorySchool {
    #[serde(default)]
    pub org_id: Option<String>,
    #[serde(default)]
    pub short_code: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub state_code: Option<String>,
    #[serde(default)]
    pub address: Option<String>,
    #[serde(default)]
    pub address2: Option<String>,
    #[serde(default)]
    pub zip: Option<String>,
    #[serde(skip)]
    pub postal_issues: Vec<String>,
    #[serde(default)]
    pub competition_levels: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchoolSummary {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub short_code: String,
    #[serde(default)]
    pub state_code: String,
    #[serde(default)]
    pub address: SummaryAddress,
    #[serde(default)]
    pub tel: Vec<SummaryTel>,
    #[serde(default)]
    pub competition_levels: BTreeMap<String, serde_json::Value>,
    #[serde(default)]
    pub staff: Vec<StaffMember>,
    #[serde(default)]
    pub teams: Vec<TeamEntry>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(from = "postal::SummaryWire")]
pub struct SummaryAddress {
    #[serde(default)]
    pub address1: Option<String>,
    #[serde(default)]
    pub address2: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(skip)]
    pub postal_issues: Vec<String>,
    #[serde(default)]
    pub city: Option<String>,
    #[serde(default)]
    pub zip: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SummaryTel {
    #[serde(default)]
    pub num: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffMember {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub amr_id: Option<String>,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub emails: Vec<String>,
    #[serde(default)]
    pub tel: Vec<SummaryTel>,
    #[serde(default)]
    pub team_name: Option<String>,
    #[serde(default)]
    pub team_level: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamEntry {
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub level: Option<String>,
    #[serde(default)]
    pub coach_profile_ids: Vec<String>,
}

pub fn parse_directory(body: &[u8]) -> CrawlResult<DirectoryPage> {
    check_body(body, "DragonFly association directory page")?;
    let page: DirectoryPage =
        serde_json::from_slice(body).map_err(|source| CrawlError::Decode {
            url: "DragonFly association directory page".to_string(),
            source,
        })?;
    if page.results.len() > MAX_SOURCE_ROWS {
        return Err(CrawlError::Schema {
            url: "DragonFly association directory page".to_string(),
            detail: format!("directory exceeds {MAX_SOURCE_ROWS} school rows"),
        });
    }
    Ok(page)
}

pub fn parse_summary(body: &[u8]) -> CrawlResult<SchoolSummary> {
    check_body(body, "DragonFly school summary")?;
    let summary: SchoolSummary =
        serde_json::from_slice(body).map_err(|source| CrawlError::Decode {
            url: "DragonFly school summary".to_string(),
            source,
        })?;
    if summary.staff.len() > MAX_SOURCE_ROWS || summary.teams.len() > MAX_SOURCE_ROWS {
        return Err(CrawlError::Schema {
            url: "DragonFly school summary".to_string(),
            detail: format!("summary exceeds {MAX_SOURCE_ROWS} staff or team rows"),
        });
    }
    Ok(summary)
}

fn check_body(body: &[u8], url: &str) -> CrawlResult<()> {
    if body.len() > crate::net::MAX_BODY_BYTES {
        return Err(CrawlError::Schema {
            url: url.to_string(),
            detail: "oversized DragonFly response".to_string(),
        });
    }
    Ok(())
}
