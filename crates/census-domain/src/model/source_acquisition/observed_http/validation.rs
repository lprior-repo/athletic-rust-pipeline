use super::{CaptureDomainError, ObservedHttpHeaderV1};
use chrono::{DateTime, Datelike, FixedOffset};
use serde::Deserializer;

pub(super) fn url(value: &str) -> Result<url::Url, CaptureDomainError> {
    super::super::scalar::bounded(value, "observed URL", 2048)?;
    super::super::scalar::url(value)
}

pub(super) fn timestamp(value: &str) -> Result<DateTime<FixedOffset>, CaptureDomainError> {
    super::super::scalar::bounded(value, "observed timestamp", 64)?;
    let parsed = DateTime::parse_from_rfc3339(value).map_err(|_| CaptureDomainError::Invalid {
        field: "observed timestamp",
    })?;
    if parsed.offset().local_minus_utc() != 0 || !(1900..=2100).contains(&parsed.year()) {
        return Err(CaptureDomainError::Invalid {
            field: "observed timestamp",
        });
    }
    Ok(parsed)
}

pub(super) fn headers(headers: &[ObservedHttpHeaderV1]) -> Result<(), CaptureDomainError> {
    if headers.len() > 32 {
        return Err(CaptureDomainError::Bound {
            field: "observed header occurrences",
            max: 32,
        });
    }
    Ok(())
}

pub(super) fn identity_encoding(
    headers: &[ObservedHttpHeaderV1],
) -> Result<(), CaptureDomainError> {
    for header in headers
        .iter()
        .filter(|header| header.name() == "content-encoding")
    {
        let value = header.value().trim_ascii();
        if !value.eq_ignore_ascii_case(b"identity")
            || header
                .value()
                .iter()
                .any(|byte| byte.is_ascii_control() && *byte != b'\t')
        {
            return Err(CaptureDomainError::Invalid {
                field: "observed content encoding",
            });
        }
    }
    Ok(())
}

pub(super) fn deserialize_headers<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<ObservedHttpHeaderV1>, D::Error> {
    crate::model::serde_bounds::sequence::<D, ObservedHttpHeaderV1, 32>(deserializer)
}
