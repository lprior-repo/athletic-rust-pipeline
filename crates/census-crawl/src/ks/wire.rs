use crate::{CrawlError, CrawlResult};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct KshsaaRecord {
    #[serde(rename = "Id")]
    pub id: u64,
    #[serde(rename = "Identifier")]
    pub identifier: String,
    #[serde(rename = "SchoolName")]
    pub school_name: String,
    #[serde(rename = "MailingCity")]
    pub mailing_city: String,
    #[serde(rename = "Class")]
    #[serde(default)]
    pub class: Option<String>,
    #[serde(rename = "Enrollment")]
    #[serde(default)]
    pub enrollment: Option<u32>,
    #[serde(rename = "WebSite")]
    #[serde(default)]
    pub web_site: Option<String>,
    #[serde(rename = "ADName")]
    #[serde(default)]
    pub ad_name: Option<String>,
    #[serde(rename = "ADEmail")]
    #[serde(default)]
    pub ad_email: Option<String>,
    #[serde(rename = "ADCell")]
    #[serde(default)]
    pub ad_cell: Option<String>,
    #[serde(rename = "PrincipalName")]
    #[serde(default)]
    pub principal_name: Option<String>,
}

pub fn parse_records(body: &str) -> CrawlResult<Vec<KshsaaRecord>> {
    let records: Vec<KshsaaRecord> =
        serde_json::from_str(body).map_err(|source| CrawlError::Decode {
            url: "KSHSAA directory JSON".to_string(),
            source,
        })?;
    Ok(records)
}
