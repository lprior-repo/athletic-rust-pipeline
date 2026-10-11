use crate::{CrawlError, CrawlResult};
use serde::Deserialize;
use std::collections::BTreeMap;

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
#[serde(rename_all = "camelCase")]
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
#[serde(rename_all = "camelCase")]
pub struct SummaryAddress {
    #[serde(default)]
    pub address1: Option<String>,
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
    serde_json::from_slice(body).map_err(|source| CrawlError::Decode {
        url: "DragonFly association directory page".to_string(),
        source,
    })
}

pub fn parse_summary(body: &[u8]) -> CrawlResult<SchoolSummary> {
    serde_json::from_slice(body).map_err(|source| CrawlError::Decode {
        url: "DragonFly school summary".to_string(),
        source,
    })
}
