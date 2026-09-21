//! Published-payload decoding: the wire shapes both IHSA endpoints return, their envelope
//! decoders, and the address parser for the per-person email reveal.
//!
//! No store access and no canonical mapping: entities are minted in [`super::map`].

use crate::sources::{CrawlError, CrawlResult};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

/// Envelope returned by `GET /v1/schools`.
#[derive(Debug, Clone, Deserialize)]
pub struct SchoolsEnvelope {
    pub data: Vec<SchoolRecord>,
}

/// One row from `/v1/schools`.
///
/// We only deserialize the fields we need. Everything else is silently ignored — including
/// address, latitude/longitude, color, and other fields that some sources publish but this
/// adapter must never touch.
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
    // Fields deliberately not used:
    // Address, POBox, Zip, Latitude, Longitude, schoolLogo, Color1/2,
    // TextOnColor1/2, HasColors, ColorSource.
}

/// One person from the `/staff2` endpoint.
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
    // Fields deliberately not used: phone, fax.
}

/// People named by a `GET /v1/schools/{id}/staff2` payload.
///
/// Staff arrive under `data`, grouped by category (`"Administration"`, `"Boys Athletics - Head
/// Coaches"`, …). The category list belongs to the association, not to us, so every array under
/// `data` is flattened whatever its label. A person holds several roles and can be listed under
/// several titles ("Boys Athletic Director" and `"IHSA Official Representative"` are the same
/// administrator here), so identity is `(PersonID, DefaultTitle)`: every role survives, exact
/// repeats do not.
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

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Non-empty trimmed string → `Some`, or `None`.
pub(super) fn nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Parse the JSON envelope returned by `GET /v1/schools`.
pub fn parse_schools(body: &str) -> CrawlResult<Vec<SchoolRecord>> {
    let envelope: SchoolsEnvelope =
        serde_json::from_str(body).map_err(|source| CrawlError::Decode {
            url: "IHSA schools JSON envelope".to_string(),
            source,
        })?;
    Ok(envelope.data)
}

/// Parse the body of `GET /v1/schools/{id}/staff/{PersonID}/email` (`{"email":"…"}`).
///
/// The endpoint is the school directory's "Show email" reveal. An empty or placeholder address
/// yields `None` so the caller never stores a non-address.
pub fn parse_email(body: &str) -> Option<String> {
    #[derive(Deserialize)]
    struct EmailEnvelope {
        #[serde(default)]
        email: Option<String>,
    }

    let envelope: EmailEnvelope = serde_json::from_str(body).ok()?;
    envelope.email.as_deref().and_then(nonempty)
}
