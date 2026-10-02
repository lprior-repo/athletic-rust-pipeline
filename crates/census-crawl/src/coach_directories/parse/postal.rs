use super::{DirectorySchool, SummaryAddress};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DirectoryWire {
    #[serde(default)]
    org_id: Option<String>,
    #[serde(default)]
    short_code: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    competition_levels: BTreeMap<String, Value>,
    #[serde(default)]
    address: PublishedText,
    #[serde(default)]
    address2: PublishedText,
    #[serde(default)]
    city: PublishedText,
    #[serde(default)]
    state_code: PublishedText,
    #[serde(default)]
    zip: PublishedText,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub(super) enum SummaryWire {
    Address(SummaryFields),
    Invalid(Value),
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SummaryFields {
    #[serde(default)]
    address1: PublishedText,
    #[serde(default)]
    address2: PublishedText,
    #[serde(default)]
    city: PublishedText,
    #[serde(default)]
    state: PublishedText,
    #[serde(default)]
    zip: PublishedText,
    #[serde(flatten)]
    _extra: BTreeMap<String, serde::de::IgnoredAny>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum PublishedText {
    Text(String),
    Other(Value),
}

impl Default for PublishedText {
    fn default() -> Self {
        Self::Other(Value::Null)
    }
}

impl PublishedText {
    fn read(self, field: &'static str) -> (Option<String>, Option<String>) {
        match self {
            Self::Text(value) if value.chars().any(char::is_control) => {
                (None, Some(format!("{field} contains control characters")))
            }
            Self::Text(value) => (Some(value), None),
            Self::Other(Value::Null) => (None, None),
            Self::Other(_) => (
                None,
                Some(format!("{field} must be published text or null")),
            ),
        }
    }
}

impl From<DirectoryWire> for DirectorySchool {
    fn from(raw: DirectoryWire) -> Self {
        let (address, street_issue) = raw.address.read("directory.address");
        let (address2, second_issue) = raw.address2.read("directory.address2");
        let (city, city_issue) = raw.city.read("directory.city");
        let (state_code, state_issue) = raw.state_code.read("directory.stateCode");
        let (zip, zip_issue) = raw.zip.read("directory.zip");
        Self {
            org_id: raw.org_id,
            short_code: raw.short_code,
            name: raw.name,
            city,
            state_code,
            address,
            address2,
            zip,
            postal_issues: [
                street_issue,
                second_issue,
                city_issue,
                state_issue,
                zip_issue,
            ]
            .into_iter()
            .flatten()
            .collect(),
            competition_levels: raw.competition_levels,
        }
    }
}

impl From<SummaryWire> for SummaryAddress {
    fn from(raw: SummaryWire) -> Self {
        let raw = match raw {
            SummaryWire::Address(raw) => raw,
            SummaryWire::Invalid(Value::Null) => return Self::default(),
            SummaryWire::Invalid(value) => {
                drop(value);
                return Self {
                    postal_issues: vec!["summary.address must be a published object".to_string()],
                    ..Self::default()
                };
            }
        };
        let (address1, street_issue) = raw.address1.read("summary.address.address1");
        let (address2, second_issue) = raw.address2.read("summary.address.address2");
        let (city, city_issue) = raw.city.read("summary.address.city");
        let (state, state_issue) = raw.state.read("summary.address.state");
        let (zip, zip_issue) = raw.zip.read("summary.address.zip");
        Self {
            address1,
            address2,
            city,
            state,
            zip,
            postal_issues: [
                street_issue,
                second_issue,
                city_issue,
                state_issue,
                zip_issue,
            ]
            .into_iter()
            .flatten()
            .collect(),
        }
    }
}
