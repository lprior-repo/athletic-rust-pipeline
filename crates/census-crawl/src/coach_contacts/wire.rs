//! The dataset's wire shapes: one CSV row, the entities a row yields, and the source coordinates
//! (namespace, provider key, URL, evidence reference) a row carries.

use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, SourceNamespace, SourceRef,
};
use serde::Deserialize;

use super::parse::{clean, nonempty};

/// One row of the contact dataset. Field names are the published CSV header.
#[derive(Debug, Clone, Deserialize)]
pub struct CoachContactRow {
    pub school: String,
    #[serde(default)]
    pub city: String,
    pub state: String,
    #[serde(default)]
    pub sport: String,
    #[serde(default)]
    pub role: String,
    #[serde(default)]
    pub coach_name: String,
    #[serde(default)]
    pub public_professional_email: String,
    #[serde(default)]
    pub ad_name: String,
    #[serde(default)]
    pub ad_email: String,
    #[serde(default)]
    pub source_url: String,
    #[serde(default)]
    pub last_observed: String,
}

/// A school plus the coaches derived from one CSV row.
#[derive(Debug, Clone)]
pub struct RowEntities {
    pub school: CanonicalSchool,
    pub coaches: Vec<CanonicalCoach>,
}

/// The association namespace a captured contact URL belongs to, so that the school identity carries
/// the origin's own key space rather than a bare name.
fn namespace_for_url(url: &str) -> SourceNamespace {
    let host = url
        .split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    let association = if host.contains("wiaawi") {
        "wiaa"
    } else if host.contains("kshsaa") {
        "kshsaa"
    } else if host.contains("ihsa") {
        "ihsa"
    } else if host.contains("myohsaa") {
        "ohsaa"
    } else if host.contains("mshsl") {
        "mshsl"
    } else if host.contains("mhsaa") {
        "mhsaa"
    } else if host.contains("nsaahome") {
        "nsaa"
    } else if host.contains("ndhsaa") {
        "ndhsaa"
    } else if host.contains("gobound") {
        return SourceNamespace::Other("bound".to_string());
    } else {
        return SourceNamespace::Other("coach_contacts_csv".to_string());
    };
    SourceNamespace::AssociationSchool {
        association: association.to_string(),
    }
}

/// Provider key extracted from a captured URL when the provider uses one (WIAA `orgID`, IHSA
/// `/schools/<id>/…`). `None` means the provider is name-keyed, which is recorded as such.
fn identity_key(url: &str) -> Option<String> {
    let query_key = |name: &str| -> Option<String> {
        url.split(['?', '&'])
            .find_map(|part| part.strip_prefix(&format!("{name}=")))
            .map(|value| value.split(['&', '#']).next().unwrap_or(value).to_string())
    };
    if let Some(value) = query_key("orgID") {
        return Some(value);
    }
    if let Some(value) = query_key("s") {
        return Some(value);
    }
    let segments: Vec<&str> = url.split('/').collect();
    for (index, segment) in segments.iter().enumerate() {
        if *segment == "schools" || *segment == "school" {
            if let Some(candidate) = segments.get(index.saturating_add(1)) {
                let candidate = candidate.trim();
                if !candidate.is_empty() && candidate.chars().all(|ch| ch.is_ascii_digit()) {
                    return Some(candidate.to_string());
                }
            }
        }
    }
    None
}

/// The source coordinates one CSV row carries: when it was observed, the provider namespace and
/// identity key, the row's URL, and the evidence reference every entity cites.
pub(super) struct RowSource {
    pub(super) observed_on: String,
    pub(super) namespace: SourceNamespace,
    pub(super) key: String,
    pub(super) url: Option<String>,
    pub(super) source_ref: SourceRef,
}

impl RowSource {
    /// Read one row's source coordinates; `school_name` keys the identity when the row has no URL.
    pub(super) fn of(row: &CoachContactRow, default_observed_on: &str, school_name: &str) -> Self {
        let observed_on = if row.last_observed.trim().is_empty() {
            default_observed_on.to_string()
        } else {
            clean(&row.last_observed)
        };
        let url = nonempty(&row.source_url);
        let namespace = url
            .as_deref()
            .map(namespace_for_url)
            .unwrap_or_else(|| SourceNamespace::Other("coach_contacts_csv".to_string()));
        let key = url
            .as_deref()
            .and_then(identity_key)
            .unwrap_or_else(|| normalize_name(school_name));
        Self {
            source_ref: SourceRef::new("coach_contacts_csv", url.clone()),
            observed_on,
            namespace,
            key,
            url,
        }
    }
}
