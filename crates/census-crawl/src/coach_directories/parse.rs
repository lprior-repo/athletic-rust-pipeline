use crate::{CrawlError, CrawlResult};
use serde::Deserialize;
use std::collections::BTreeMap;

mod postal;

const MAX_SOURCE_ROWS: usize = 20_000;

/// Parse the directory JSON body and validate its shape.
/// A JSON-valid but shape-divergent body (e.g. auth error object) is rejected.
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
    validate_directory_shape(body, &page)?;
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
/// Reject bodies that parse to all-default values because they are
/// shape-divergent (e.g. {"error":"unauthorized"}) rather than legitimately
/// empty. A legitimately empty page carries at least one expected field.
fn validate_directory_shape(body: &[u8], page: &DirectoryPage) -> CrawlResult<()> {
    let all_defaults = page.current_page == 0
        && page.total_pages == 0
        && page.total_results == 0
        && page.results.is_empty();
    if !all_defaults {
        return Ok(());
    }
    // All defaults — check if the body actually contained expected field names.
    // A legitimately empty page has at least one of these.
    let body_str = String::from_utf8_lossy(body);
    let has_expected_field =
        body_str.contains("currentPage") || body_str.contains("totalPages") || body_str.contains("totalResults");
    if !has_expected_field {
        return Err(CrawlError::Schema {
            url: "DragonFly association directory page".to_string(),
            detail: format!(
                "shape-divergent directory body (no expected fields found): {}",
                String::from_utf8_lossy(body).chars().take(200).collect::<String>()
            ),
        });
    }
    Ok(())
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
