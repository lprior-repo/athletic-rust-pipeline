use std::collections::BTreeSet;

use super::MAX_EVIDENCE_TEXT_BYTES;
use super::{value_error, Acquisition, EvidenceInput, JoinError, MAX_EVIDENCE_RECORDS};

#[derive(Default)]
pub(super) struct TextBudget {
    bytes: usize,
}

impl TextBudget {
    pub(super) fn charge(&mut self, value: &str) -> Result<(), JoinError> {
        self.bytes = self.bytes.checked_add(value.len()).ok_or_else(|| {
            value_error("bytes", "evidence", "evidence byte accounting overflowed")
        })?;
        if self.bytes > MAX_EVIDENCE_TEXT_BYTES {
            return Err(value_error(
                "bytes",
                "evidence",
                "evidence exceeds 4194304 text bytes",
            ));
        }
        Ok(())
    }
}

pub(super) fn require_count(kind: &'static str, count: usize) -> Result<(), JoinError> {
    if count > MAX_EVIDENCE_RECORDS {
        return Err(value_error(
            kind,
            &count.to_string(),
            "expected at most 256 records",
        ));
    }
    Ok(())
}

pub(super) fn require_selector(source: &str) -> Result<(), JoinError> {
    match source.split_once('@') {
        Some((provider, path)) if path.is_empty() || provider.is_empty() => Err(value_error(
            "selector",
            source,
            "capture selector requires provider and path",
        )),
        Some((provider, _)) => require_provider(provider),
        None => require_provider(source),
    }
}

fn require_provider(provider: &str) -> Result<(), JoinError> {
    if !provider.contains('@') && super::super::admitted_provider(provider) {
        return Ok(());
    }
    Err(JoinError::EvidenceSource {
        provider: provider.to_string(),
    })
}

pub(super) fn validate_acquisition(value: &Acquisition) -> Result<(), JoinError> {
    let mut budget = TextBudget::default();
    charge_acquisition(&mut budget, value)?;
    require_provider(&value.provider)?;
    validate_url(&value.provider, &value.url)?;
    if let Some(date) = &value.retrieved_at {
        validate_date(&value.provider, date)?;
    }
    Ok(())
}

pub(super) fn validate_input(input: &EvidenceInput) -> Result<(), JoinError> {
    let mut budget = TextBudget::default();
    match input {
        EvidenceInput::Acquisitions(values) => validate_acquisitions(values, &mut budget),
        EvidenceInput::LegacySourcePairs(values) => {
            validate_pairs("url", &values.urls, &mut budget)?;
            validate_pairs("date", &values.dates, &mut budget)
        }
    }
}

fn validate_acquisitions(values: &[Acquisition], budget: &mut TextBudget) -> Result<(), JoinError> {
    require_count("acquisitions", values.len())?;
    let mut entries = BTreeSet::new();
    for value in values {
        if !entries.insert(value.entry) {
            return Err(value_error(
                "entry",
                &value.entry.to_string(),
                "duplicate acquisition entry",
            ));
        }
        charge_acquisition(budget, value)?;
        validate_acquisition(value)?;
    }
    Ok(())
}

fn charge_acquisition(budget: &mut TextBudget, value: &Acquisition) -> Result<(), JoinError> {
    budget.charge(&value.provider)?;
    budget.charge(&value.path)?;
    budget.charge(&value.url)?;
    if let Some(date) = &value.retrieved_at {
        budget.charge(date)?;
    }
    Ok(())
}

fn validate_pairs(
    kind: &'static str,
    values: &std::collections::BTreeMap<String, String>,
    budget: &mut TextBudget,
) -> Result<(), JoinError> {
    require_count(kind, values.len())?;
    for (source, value) in values {
        budget.charge(source)?;
        budget.charge(value)?;
        require_selector(source)?;
        match kind {
            "url" => validate_url(source, value)?,
            "date" => validate_date(source, value)?,
            _ => return Err(value_error("kind", kind, "unsupported evidence value kind")),
        }
    }
    Ok(())
}

fn validate_url(source: &str, value: &str) -> Result<(), JoinError> {
    match url::Url::parse(value) {
        Ok(parsed) if matches!(parsed.scheme(), "http" | "https") => Ok(()),
        Ok(_) => Err(value_error(
            "url",
            &format!("{source}={value}"),
            "url scheme must be http or https",
        )),
        Err(error) => Err(value_error(
            "url",
            &format!("{source}={value}"),
            &error.to_string(),
        )),
    }
}

fn validate_date(source: &str, value: &str) -> Result<(), JoinError> {
    let trimmed = value.trim();
    let valid = if trimmed.len() == 10 {
        chrono::NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").is_ok()
    } else {
        chrono::DateTime::parse_from_rfc3339(trimmed).is_ok()
    };
    if valid {
        return Ok(());
    }
    Err(value_error(
        "date",
        &format!("{source}={value}"),
        "expected a YYYY-MM-DD date or RFC3339 timestamp",
    ))
}
