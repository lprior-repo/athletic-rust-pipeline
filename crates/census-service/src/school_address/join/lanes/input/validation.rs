use super::{
    value_error, Acquisition, EvidenceInput, JoinError, MAX_EVIDENCE_RECORDS,
    MAX_EVIDENCE_TEXT_BYTES,
};
use std::collections::BTreeSet;

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
