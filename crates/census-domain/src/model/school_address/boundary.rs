use super::SchoolAddressError;
use crate::model::{Evidence, EvidenceMethod, SourceIdentity, SourceNamespace};
use crate::school_directory::SourceLabel;
use crate::UsJurisdiction;

pub(super) fn provenance(
    owner: &SourceIdentity,
    label: &SourceLabel,
    evidence: &Evidence,
) -> Result<UsJurisdiction, SchoolAddressError> {
    let state = claim_state(owner, label)?;
    let expected = owner_source(&owner.namespace).ok_or(SchoolAddressError::MissingOwner)?;
    if evidence.source.id != expected {
        return Err(SchoolAddressError::SourceAuthorityMismatch);
    }
    if evidence.method != EvidenceMethod::Parsed || evidence.observed_on.is_empty() {
        return Err(SchoolAddressError::MissingParsedCapture);
    }
    if !observation_date(&evidence.observed_on) {
        return Err(SchoolAddressError::InvalidObservationDate);
    }
    let url = evidence
        .source
        .url
        .as_deref()
        .ok_or(SchoolAddressError::MissingParsedCapture)?;
    if !source_url(url) || owner.url.as_deref().is_some_and(|url| !source_url(url)) {
        return Err(SchoolAddressError::InvalidSourceUrl);
    }
    if evidence
        .note
        .as_deref()
        .is_some_and(|note| !single_line(note, 4096))
    {
        return Err(SchoolAddressError::InvalidProvenance);
    }
    Ok(state)
}

pub(super) fn claim_state(
    owner: &SourceIdentity,
    label: &SourceLabel,
) -> Result<UsJurisdiction, SchoolAddressError> {
    match &owner.namespace {
        SourceNamespace::AssociationSchool { association } => {
            if !owner_bounds(owner, association) {
                return Err(SchoolAddressError::MissingOwner);
            }
            source_state(label)
        }
        SourceNamespace::SchoolDirectory { provider, state } => {
            if !owner_bounds(owner, provider) {
                return Err(SchoolAddressError::MissingOwner);
            }
            if directory_source(label) != Some(provider.as_str()) {
                return Err(SchoolAddressError::UnsupportedAuthority);
            }
            Ok(*state)
        }
        _ => Err(SchoolAddressError::MissingOwner),
    }
}

pub(super) fn source_state(label: &SourceLabel) -> Result<UsJurisdiction, SchoolAddressError> {
    match label {
        SourceLabel::AthleticAssociation { state } => Ok(*state),
        SourceLabel::Ccd
        | SourceLabel::Pss
        | SourceLabel::StateEducationAgency { .. }
        | SourceLabel::PrivateAssociation { .. }
        | SourceLabel::Geocoder => Err(SchoolAddressError::UnsupportedAuthority),
    }
}

fn owner_source(namespace: &SourceNamespace) -> Option<&str> {
    match namespace {
        SourceNamespace::AssociationSchool { association } => Some(association),
        SourceNamespace::SchoolDirectory { provider, .. } => Some(provider),
        _ => None,
    }
}

fn directory_source(label: &SourceLabel) -> Option<&'static str> {
    match label {
        SourceLabel::Ccd => Some("nces-ccd"),
        SourceLabel::Pss => Some("nces-pss"),
        SourceLabel::StateEducationAgency { .. }
        | SourceLabel::PrivateAssociation { .. }
        | SourceLabel::AthleticAssociation { .. }
        | SourceLabel::Geocoder => None,
    }
}

fn owner_bounds(owner: &SourceIdentity, token: &str) -> bool {
    single_line(&owner.id, 256)
        && !owner.id.chars().any(char::is_whitespace)
        && canonical_source(token)
}

fn canonical_source(value: &str) -> bool {
    single_line(value, 128)
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

fn single_line(value: &str, limit: usize) -> bool {
    !value.trim().is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}

fn observation_date(value: &str) -> bool {
    if !single_line(value, 40) || value.trim() != value {
        return false;
    }
    if value.len() == 10 {
        return chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
            .is_ok_and(|date| (1900..=2100).contains(&chrono::Datelike::year(&date)));
    }
    chrono::DateTime::parse_from_rfc3339(value)
        .is_ok_and(|date| (1900..=2100).contains(&chrono::Datelike::year(&date)))
}

fn source_url(value: &str) -> bool {
    single_line(value, 2048)
        && !value.chars().any(char::is_whitespace)
        && url::Url::parse(value).is_ok_and(|url| {
            matches!(url.scheme(), "http" | "https")
                && url.host_str().is_some()
                && url.username().is_empty()
                && url.password().is_none()
                && url.port() != Some(0)
        })
}
