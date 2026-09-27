
use crate::{CrawlError, CrawlResult};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Deserialize)]
pub struct SchoolsEnvelope {
    pub data: Vec<SchoolRecord>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SchoolRecord {
    #[serde(rename = "SchoolID")]
    pub school_id: String,
    #[serde(rename = "nameFormal")]
    pub name_formal: String,
    #[serde(rename = "NameIHSA")]
    #[serde(default)]
    pub name_ihsa: Option<String>,
    #[serde(default)]
    pub name_short: Option<String>,
    pub city: String,
    #[serde(rename = "membershipType")]
    #[serde(default)]
    pub membership_type: Option<String>,
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(rename = "enrollmentType")]
    #[serde(default)]
    pub enrollment_type: Option<String>,
    #[serde(rename = "hasBoundary")]
    #[serde(default)]
    pub has_boundary: Option<String>,
    #[serde(rename = "isCPS")]
    #[serde(default)]
    pub is_cps: Option<String>,
    #[serde(rename = "URL")]
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct StaffPerson {
    #[serde(rename = "PersonID")]
    pub person_id: i64,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "DefaultTitle")]
    pub default_title: String,
    #[serde(rename = "HasEmail")]
    #[serde(default)]
    pub has_email: Option<bool>,
    #[serde(rename = "LastName")]
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(rename = "RoleID")]
    #[serde(default)]
    pub role_id: Option<String>,
    #[serde(rename = "Phone")]
    #[serde(default)]
    pub phone: Option<String>,
    #[serde(rename = "Fax")]
    #[serde(default)]
    pub fax: Option<String>,
    #[serde(rename = "email")]
    #[serde(default)]
    pub email: Option<String>,
}

pub fn parse_staff(body: &str) -> CrawlResult<Vec<StaffPerson>> {
    #[derive(Deserialize)]
    struct Envelope {
        data: BTreeMap<String, Vec<StaffPerson>>,
    }

    let envelope: Envelope = serde_json::from_str(body).map_err(|source| CrawlError::Decode {
        url: "IHSA staff JSON envelope".to_string(),
        source,
    })?;
    let mut seen = BTreeSet::new();
    let mut people = Vec::new();
    for (_, mut category) in envelope.data {
        category.retain(|person| seen.insert((person.person_id, person.default_title.clone())));
        people.append(&mut category);
    }
    Ok(people)
}

pub(super) fn nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn parse_schools(body: &str) -> CrawlResult<Vec<SchoolRecord>> {
    let envelope: SchoolsEnvelope =
        serde_json::from_str(body).map_err(|source| CrawlError::Decode {
            url: "IHSA schools JSON envelope".to_string(),
            source,
        })?;
    Ok(envelope.data)
}

pub fn parse_email(body: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct EmailEnvelope {
        #[serde(default)]
        email: Option<String>,
    }

    let envelope: EmailEnvelope = serde_json::from_str(body).ok()?;
    envelope.email.as_deref().and_then(nonempty)
}
