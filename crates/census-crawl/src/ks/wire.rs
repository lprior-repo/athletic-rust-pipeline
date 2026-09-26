//! The KSHSAA directory JSON wire shape and its reader.
use crate::{CrawlError, CrawlResult};
use serde::Deserialize;

/// One record from the KSHSAA directory JSON.
///
/// We only deserialize the fields we need. Everything else is silently ignored — including phone
/// fields that some sources publish but this adapter must never touch.
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

/// Parse the JSON envelope returned by the KSHSAA directory API.
///
/// The response is a flat JSON array — no nesting. Returns the parsed records in API order.
pub fn parse_records(body: &str) -> CrawlResult<Vec<KshsaaRecord>> {
    let records: Vec<KshsaaRecord> =
        serde_json::from_str(body).map_err(|source| CrawlError::Decode {
            url: "KSHSAA directory JSON".to_string(),
            source,
        })?;
    Ok(records)
}
