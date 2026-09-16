use super::AthleteId;
use crate::domain::error::DomainError;
use url::Url;

const SHA256_HEX_LENGTH: usize = 64;
const MAX_SHEET_BYTES: usize = 128;
const ATHLETIC_HOST: &str = "athletic.net";

pub(super) fn validate_digest(raw: &str, field: &'static str) -> Result<String, DomainError> {
    if raw.len() != SHA256_HEX_LENGTH {
        return Err(DomainError::InvalidFormat { field });
    }
    if raw
        .bytes()
        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        Ok(raw.to_owned())
    } else {
        Err(DomainError::InvalidFormat { field })
    }
}

pub(super) fn validate_sheet(sheet: &str) -> Result<(), DomainError> {
    if sheet.is_empty() {
        return Err(DomainError::Empty {
            field: "source_row_key_sheet",
        });
    }
    if sheet.len() > MAX_SHEET_BYTES {
        return Err(DomainError::TooLong {
            field: "source_row_key_sheet",
            limit: MAX_SHEET_BYTES,
        });
    }
    if sheet.chars().any(char::is_control) {
        return Err(DomainError::InvalidFormat {
            field: "source_row_key_sheet",
        });
    }
    Ok(())
}

pub(super) fn validate_profile_origin(raw: &str, parsed: &Url) -> Result<(), DomainError> {
    if raw.contains('\\')
        || parsed.scheme() != "https"
        || !supported_host(parsed.host_str())
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || authority_contains_at(raw)
        || parsed.port().is_some_and(|port| port != 443)
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        return Err(DomainError::InvalidFormat {
            field: "profile_url",
        });
    }
    Ok(())
}

fn supported_host(host: Option<&str>) -> bool {
    host == Some(ATHLETIC_HOST) || host == Some("www.athletic.net")
}

fn authority_contains_at(raw: &str) -> bool {
    raw.split_once("://")
        .and_then(|(_, rest)| rest.split(['/', '?', '#']).next())
        .is_some_and(|authority| authority.contains('@'))
}

pub(super) fn profile_path(path: &str) -> Result<(&str, &'static str), DomainError> {
    let normalized = path.strip_suffix('/').map_or(path, |value| value);
    let mut parts = normalized.split('/');
    validate_profile_prefix(&mut parts)?;
    let id = parts.next().ok_or(DomainError::InvalidFormat {
        field: "profile_url_athlete_id",
    })?;
    validate_profile_id(id)?;
    let suffix = profile_suffix(&mut parts)?;
    if parts.next().is_some() {
        return Err(DomainError::Unsupported {
            field: "profile_url_path",
        });
    }
    Ok((id, suffix))
}

fn validate_profile_prefix(parts: &mut std::str::Split<'_, char>) -> Result<(), DomainError> {
    if parts.next() == Some("") && parts.next() == Some("athlete") {
        Ok(())
    } else {
        Err(DomainError::Unsupported {
            field: "profile_url_path",
        })
    }
}

fn validate_profile_id(id: &str) -> Result<(), DomainError> {
    if !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()) {
        Ok(())
    } else {
        Err(DomainError::InvalidFormat {
            field: "profile_url_athlete_id",
        })
    }
}

fn profile_suffix(parts: &mut std::str::Split<'_, char>) -> Result<&'static str, DomainError> {
    match (parts.next(), parts.next()) {
        (None, None) => Ok(""),
        (Some("track-and-field"), None) => Ok("/track-and-field"),
        (Some("cross-country"), None) => Ok("/cross-country"),
        (Some("track-and-field"), Some("all")) => Ok("/track-and-field/all"),
        (Some("cross-country"), Some("all")) => Ok("/cross-country/all"),
        _ => Err(DomainError::Unsupported {
            field: "profile_url_path",
        }),
    }
}

pub(super) fn canonical_profile_url(athlete_id: AthleteId, suffix: &str) -> String {
    format!(
        "https://{ATHLETIC_HOST}/athlete/{}{suffix}",
        athlete_id.get()
    )
}
