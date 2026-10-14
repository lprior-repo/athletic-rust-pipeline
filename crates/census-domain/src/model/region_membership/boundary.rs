use super::{RegionAssertionV1, RegionEvidenceInputV1, RegionMembershipContractInputV1};
use super::{RegionMembershipError, RegionSchoolV1, MAX_REGION_ASSERTIONS};
use super::{MAX_REGION_CONSTITUENTS, MAX_REGION_JURISDICTIONS, MAX_REGION_PAYLOAD_BYTES};
use crate::model::{EvidenceMethod, SchoolId, TeamId};
use chrono::Datelike;
use serde::Deserializer;

pub(super) fn count(
    field: &'static str,
    observed: usize,
    maximum: usize,
) -> Result<(), RegionMembershipError> {
    if observed > maximum {
        return Err(RegionMembershipError::TooManyEntries {
            field,
            observed,
            maximum,
        });
    }
    Ok(())
}

pub(super) fn school_id(id: &SchoolId) -> Result<(), RegionMembershipError> {
    if !id.validate("sch")
        || id.as_str().len() <= 4
        || !token(id.as_str(), crate::model::MAX_LOGICAL_ID_BYTES)
    {
        return Err(RegionMembershipError::InvalidField { field: "school_id" });
    }
    Ok(())
}

pub(super) fn program_id(id: &TeamId) -> Result<(), RegionMembershipError> {
    if !id.validate("team")
        || id.as_str().len() <= 5
        || !token(id.as_str(), crate::model::MAX_LOGICAL_ID_BYTES)
    {
        return Err(RegionMembershipError::InvalidField {
            field: "program_id",
        });
    }
    Ok(())
}

pub(super) fn validate_evidence(
    input: &RegionEvidenceInputV1,
) -> Result<(), RegionMembershipError> {
    let evidence = &input.evidence;
    let valid = evidence.method == EvidenceMethod::Parsed
        && token(&evidence.source.id, 128)
        && evidence.source.url.as_deref().is_some_and(source_url)
        && observation_day_or_instant(&evidence.observed_on)
        && evidence
            .note
            .as_deref()
            .is_none_or(|note| single_line(note, 4096))
        && crate::model::engine_valid::valid_hex64(&input.capture_sha256)
        && single_line(&input.locator, 512)
        && single_line(&input.statement, 512);
    if !valid {
        return Err(RegionMembershipError::InvalidField {
            field: "provenance",
        });
    }
    Ok(())
}

fn single_line(value: &str, limit: usize) -> bool {
    value.len() <= limit
        && !value.trim().is_empty()
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn token(value: &str, limit: usize) -> bool {
    single_line(value, limit) && !value.chars().any(char::is_whitespace)
}

fn source_url(value: &str) -> bool {
    token(value, 2048)
        && url::Url::parse(value).is_ok_and(|url| {
            matches!(url.scheme(), "http" | "https")
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
                && url.port() != Some(0)
        })
}

fn observation_day_or_instant(value: &str) -> bool {
    if !token(value, 40) {
        return false;
    }
    if value.len() == 10 {
        return super::ProfileDate::parse(value).is_ok();
    }
    chrono::DateTime::parse_from_rfc3339(value)
        .is_ok_and(|date| (1900..=2100).contains(&date.year()))
}

pub(super) fn retained_bytes(
    input: &RegionMembershipContractInputV1,
) -> Result<(), RegionMembershipError> {
    let mut size = SizeWriter {
        bytes: 0,
        exceeded: false,
    };
    let result = crate::model::write_serialized(&mut size, input);
    if size.exceeded {
        return Err(RegionMembershipError::PayloadTooLarge {
            maximum: MAX_REGION_PAYLOAD_BYTES,
        });
    }
    result?;
    Ok(())
}

struct SizeWriter {
    bytes: usize,
    exceeded: bool,
}

impl std::io::Write for SizeWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        match self.bytes.checked_add(bytes.len()) {
            Some(total) if total <= MAX_REGION_PAYLOAD_BYTES => {
                self.bytes = total;
                Ok(bytes.len())
            }
            Some(_) | None => {
                self.exceeded = true;
                Err(std::io::Error::other("regional payload size ceiling"))
            }
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(super) fn assertions<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<RegionAssertionV1>, D::Error> {
    crate::model::serde_bounds::sequence::<D, RegionAssertionV1, MAX_REGION_ASSERTIONS>(
        deserializer,
    )
}

pub(super) fn jurisdictions<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<crate::UsJurisdiction>, D::Error> {
    crate::model::serde_bounds::sequence::<D, crate::UsJurisdiction, MAX_REGION_JURISDICTIONS>(
        deserializer,
    )
}

pub(super) fn constituents<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<RegionSchoolV1>, D::Error> {
    crate::model::serde_bounds::sequence::<D, RegionSchoolV1, MAX_REGION_CONSTITUENTS>(deserializer)
}
