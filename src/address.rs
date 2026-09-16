use crate::model::Prospect;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

const STREET_FIELD: &str = "Address Mailing / Permanent Street Combined";
const POSTAL_FIELD: &str = "Address Mailing / Permanent Postal";
static PO_BOX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)^P\.?\s*O\.?\s+BOX\s+([0-9]+[A-Z]?)$"));
static NUMBERED: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"^([0-9]+[A-Za-z]?)\s+(.+)$"));
static UNIT: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(.+?)\s*,?\s+(APT\.?|APARTMENT|UNIT|SUITE|STE\.?|#)\s*([A-Z0-9]+(?:-[A-Z0-9]+)?)$",
    )
});
static UNCONSUMED: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r"(?i)(\b(?:APT|APARTMENT|UNIT|SUITE|STE|BOX)\b|#|\b[0-9]+\s+\p{L})")
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AddressIssueSeverity {
    Advisory,
    Review,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressIssue {
    pub code: String,
    pub message: String,
    pub severity: AddressIssueSeverity,
}

/// Conservative syntax evidence, not ZIP/state or deliverability validation.
/// Raw values preserve whitespace, leading zeros, punctuation and absence.
/// Only address fields are copied here; email is deliberately excluded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressEvidence {
    pub raw_street_combined: Option<String>,
    pub raw_postal: Option<String>,
    pub raw_city: Option<String>,
    pub raw_state: Option<String>,
    pub street_number: Option<String>,
    pub road: Option<String>,
    pub unit: Option<String>,
    pub po_box: Option<String>,
    pub city: Option<String>,
    pub region: Option<String>,
    /// Trimmed source text, including unknown foreign or malformed formats.
    /// Consult `us_postal_syntax` and issues before interpreting this value.
    pub postal: Option<String>,
    /// Some(true/false) only when a US region was recognized and postal exists.
    /// This does not establish that a ZIP exists or matches the region.
    pub us_postal_syntax: Option<bool>,
    pub issues: Vec<AddressIssue>,
}

impl AddressEvidence {
    pub fn requires_review(&self) -> bool {
        self.issues
            .iter()
            .any(|issue| issue.severity == AddressIssueSeverity::Review)
    }

    pub fn issues(&self) -> &[AddressIssue] {
        &self.issues
    }

    fn issue(&mut self, code: &str, message: &str, severity: AddressIssueSeverity) {
        self.issues.push(AddressIssue {
            code: code.to_owned(),
            message: message.to_owned(),
            severity,
        });
    }
}

pub fn parse(prospect: &Prospect) -> AddressEvidence {
    parse_fields(
        prospect.source_fields.get(STREET_FIELD).map(String::as_str),
        prospect.source_fields.get(POSTAL_FIELD).map(String::as_str),
        Some(&prospect.city),
        Some(&prospect.state),
    )
}

/// Public field-level parser, also useful when the source row has not yet been
/// materialized as a Prospect. No country or postal reference data is inferred.
pub fn parse_fields(
    street: Option<&str>,
    postal: Option<&str>,
    city: Option<&str>,
    state: Option<&str>,
) -> AddressEvidence {
    let region = trimmed(state).and_then(|value| crate::alpha_url::canonical_state(&value));
    let mut evidence = AddressEvidence {
        raw_street_combined: street.map(str::to_owned),
        raw_postal: postal.map(str::to_owned),
        raw_city: city.map(str::to_owned),
        raw_state: state.map(str::to_owned),
        street_number: None,
        road: None,
        unit: None,
        po_box: None,
        city: trimmed(city),
        region,
        postal: trimmed(postal),
        us_postal_syntax: None,
        issues: Vec::new(),
    };
    [
        (
            trimmed(street).is_none(),
            "missing_street",
            "Street address is absent.",
        ),
        (
            evidence.postal.is_none(),
            "missing_postal",
            "Postal value is absent.",
        ),
        (evidence.city.is_none(), "missing_city", "City is absent."),
        (
            trimmed(state).is_none(),
            "missing_region",
            "Region is absent.",
        ),
    ]
    .into_iter()
    .filter(|(missing, _, _)| *missing)
    .for_each(|(_, code, message)| {
        evidence.issue(code, message, AddressIssueSeverity::Advisory);
    });
    if trimmed(state).is_some() && evidence.region.is_none() {
        evidence.issue(
            "unknown_region",
            "Region was retained raw; no US region was inferred.",
            AddressIssueSeverity::Advisory,
        );
    }
    evidence.us_postal_syntax = evidence
        .postal
        .as_deref()
        .filter(|_| evidence.region.is_some())
        .map(us_zip_shape);
    if evidence.us_postal_syntax == Some(false) {
        evidence.issue("malformed_us_postal", "Postal syntax is not five ASCII digits or ZIP+4 for the recognized US region; raw text is retained.", AddressIssueSeverity::Review);
    }
    if let Some(value) = trimmed(street) {
        parse_street(&value, &mut evidence);
    }
    evidence
}

fn trimmed(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn us_zip_shape(value: &str) -> bool {
    if value.len() == 5 {
        return value.bytes().all(|byte| byte.is_ascii_digit());
    }
    match value.split_once('-') {
        Some((base, extension)) => {
            base.len() == 5
                && extension.len() == 4
                && base.bytes().all(|byte| byte.is_ascii_digit())
                && extension.bytes().all(|byte| byte.is_ascii_digit())
        }
        None => false,
    }
}

fn expression(
    pattern: &'static LazyLock<Result<Regex, regex::Error>>,
    evidence: &mut AddressEvidence,
) -> Option<&'static Regex> {
    match pattern.as_ref() {
        Ok(regex) => Some(regex),
        Err(_) => {
            evidence.issue(
                "parser_configuration",
                "Address syntax parser could not be initialized; raw values are retained.",
                AddressIssueSeverity::Review,
            );
            None
        }
    }
}

fn ambiguous(evidence: &mut AddressEvidence) {
    evidence.issue("ambiguous_street", "Street text could not be conservatively separated into one street address or PO box; raw text is retained.", AddressIssueSeverity::Review);
}

fn parse_street(value: &str, evidence: &mut AddressEvidence) {
    // Do not silently combine multiple addresses or consume city/postal lines.
    let lower = value.to_ascii_lowercase();
    if value.chars().any(|character| character.is_control())
        || value.contains([';', '&', '/'])
        || lower.contains(" and ")
    {
        ambiguous(evidence);
        return;
    }
    let Some(box_pattern) = expression(&PO_BOX, evidence) else {
        return;
    };
    if let Some(captures) = box_pattern.captures(value) {
        evidence.po_box = captures.get(1).map(|part| part.as_str().to_owned());
        return;
    }
    let Some(numbered) = expression(&NUMBERED, evidence) else {
        return;
    };
    let Some(captures) = numbered.captures(value) else {
        ambiguous(evidence);
        return;
    };
    let Some(remainder) = captures.get(2).map(|part| part.as_str()) else {
        ambiguous(evidence);
        return;
    };
    let Some(unit_pattern) = expression(&UNIT, evidence) else {
        return;
    };
    let (road, unit) = match unit_pattern.captures(remainder) {
        Some(parts) => (
            parts
                .get(1)
                .map(|part| part.as_str().trim_end_matches(',').trim()),
            parts.get(3).map(|part| part.as_str().to_owned()),
        ),
        None => (Some(remainder), None),
    };
    let Some(road) = road else {
        ambiguous(evidence);
        return;
    };
    let Some(unconsumed) = expression(&UNCONSUMED, evidence) else {
        return;
    };
    if !road.chars().any(char::is_alphabetic)
        || road.contains([',', ':', '!'])
        || unconsumed.is_match(road)
    {
        ambiguous(evidence);
        return;
    }
    evidence.street_number = captures.get(1).map(|part| part.as_str().to_owned());
    evidence.road = Some(road.to_owned());
    evidence.unit = unit;
}

#[cfg(test)]
#[path = "address_tests.rs"]
mod tests;
