use serde::Deserialize;
use serde_json::Value;

use super::rows::{DocDivision, DocRow};
use super::{value_flag, value_u64};
use crate::{CrawlError, CrawlResult};
use census_domain::model::EventKind;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EventDoc {
    #[serde(default, rename = "i")]
    pub id: Option<Value>,
    #[serde(default, rename = "mi")]
    pub meet_id: Option<Value>,
    #[serde(default, rename = "n")]
    pub name: Option<String>,
    #[serde(default, rename = "ab")]
    pub abbrev: Option<String>,
    #[serde(default, rename = "un")]
    pub unit: Option<String>,
    #[serde(default, rename = "gl")]
    pub gender_group: Option<String>,
    #[serde(default, rename = "xc")]
    pub xc: Option<Value>,
    #[serde(default, rename = "dv")]
    pub division: Option<DocDivision>,
    #[serde(default, rename = "runm")]
    pub round_name: Option<String>,
    #[serde(default, rename = "rui")]
    pub run_id: Option<String>,
    #[serde(default, rename = "r")]
    pub rows: Vec<DocRow>,
}

impl EventDoc {
    pub fn event_id(&self) -> Option<u64> {
        self.id.as_ref().and_then(value_u64)
    }

    pub fn meet_id(&self) -> Option<u64> {
        self.meet_id.as_ref().and_then(value_u64)
    }

    pub fn is_xc(&self) -> bool {
        self.xc.as_ref().is_some_and(value_flag)
    }

    pub fn label(&self) -> Option<&str> {
        [
            self.abbrev.as_deref(),
            self.unit.as_deref(),
            self.name.as_deref(),
        ]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|label| !label.is_empty())
    }

    pub fn kind(&self) -> EventKind {
        if self.is_xc() {
            return EventKind::CrossCountry;
        }
        match self.label() {
            Some(label) => EventKind::from_source_label(label),
            None => EventKind::Unmapped {
                label: String::new(),
            },
        }
    }

    pub fn division_name(&self) -> Option<&str> {
        self.division
            .as_ref()
            .and_then(|division| division.n.as_deref())
            .map(str::trim)
            .filter(|name| !name.is_empty())
    }

    pub fn run_id(&self) -> Option<&str> {
        self.run_id
            .as_deref()
            .map(str::trim)
            .filter(|id| !id.is_empty())
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SummaryEvent {
    #[serde(default, rename = "i")]
    pub id: Option<Value>,
    #[serde(default, rename = "ec")]
    pub entry_class: Option<String>,
    #[serde(default, rename = "ab")]
    pub abbrev: Option<String>,
    #[serde(default, rename = "un")]
    pub unit: Option<String>,
    #[serde(default, rename = "n")]
    pub name: Option<String>,
    #[serde(default, rename = "peg")]
    pub family: Option<String>,
    #[serde(default, rename = "xc")]
    pub xc: Option<Value>,
}

impl SummaryEvent {
    pub fn event_id(&self) -> Option<u64> {
        self.id.as_ref().and_then(value_u64)
    }

    pub fn is_xc(&self) -> bool {
        self.xc.as_ref().is_some_and(value_flag)
    }

    pub fn is_relay(&self) -> bool {
        let entry_class = self.entry_class.as_deref().unwrap_or_default().trim();
        entry_class.eq_ignore_ascii_case("relay")
            || self
                .family
                .as_deref()
                .is_some_and(|family| family.trim().eq_ignore_ascii_case("relay"))
    }

    pub fn label(&self) -> Option<&str> {
        [
            self.abbrev.as_deref(),
            self.unit.as_deref(),
            self.name.as_deref(),
        ]
        .into_iter()
        .flatten()
        .map(str::trim)
        .find(|label| !label.is_empty())
    }

    pub fn kind(&self) -> EventKind {
        if self.is_xc() {
            return EventKind::CrossCountry;
        }
        match self.label() {
            Some(label) => EventKind::from_source_label(label),
            None => EventKind::Unmapped {
                label: String::new(),
            },
        }
    }
}

pub fn parse_event_document(url: &str, body: &str) -> CrawlResult<EventDoc> {
    #[derive(Deserialize)]
    struct Envelope {
        #[serde(default, rename = "_source")]
        source: Option<EventDoc>,
    }
    let envelope: Envelope = serde_json::from_str(body).map_err(|source| CrawlError::Decode {
        url: url.to_string(),
        source,
    })?;
    envelope.source.ok_or_else(|| CrawlError::Schema {
        url: url.to_string(),
        detail: "an event document carries its payload under `_source`".to_string(),
    })
}

pub fn parse_event_summary(url: &str, body: &str) -> CrawlResult<Vec<SummaryEvent>> {
    let events: std::collections::BTreeMap<String, SummaryEvent> = serde_json::from_str(body)
        .map_err(|source| CrawlError::Decode {
            url: url.to_string(),
            source,
        })?;
    Ok(events.into_values().collect())
}
