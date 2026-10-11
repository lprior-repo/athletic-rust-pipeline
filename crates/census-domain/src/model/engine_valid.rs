use crate::DomainError;

pub const MAX_PROVIDER_ID_BYTES: usize = 128;
pub const MAX_LOGICAL_ID_BYTES: usize = 512;
pub const MAX_ENGINE_DIGEST_BYTES: usize = 256;
pub const MAX_URL_BYTES: usize = 4096;
pub const MAX_DETAIL_BYTES: usize = 1024;
pub const MAX_EVIDENCE_DIGEST_BYTES: usize = 256;
pub const MAX_CAPTURE_BYTES: u64 = 50_000_000;
pub const MAX_MEMBERS_HINT: usize = 256;

pub fn valid_digest(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_ENGINE_DIGEST_BYTES && !value.bytes().any(|b| b == 0)
}

pub fn valid_hex64(value: &str) -> bool {
    if value.len() != 64 {
        return false;
    }
    value
        .bytes()
        .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

pub fn valid_date(value: &str) -> bool {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").is_ok()
}

pub fn valid_url(value: &str) -> bool {
    if value.is_empty() || value.len() > MAX_URL_BYTES {
        return false;
    }
    if value.bytes().any(|b| b == 0) {
        return false;
    }
    value.starts_with("https://") || value.starts_with("http://")
}

pub fn valid_id(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_LOGICAL_ID_BYTES && !value.bytes().any(|b| b == 0)
}

pub fn valid_detail(value: &str) -> bool {
    !value.is_empty() && value.len() <= MAX_DETAIL_BYTES && !value.bytes().any(|b| b == 0)
}

pub fn refuse_id(field: &'static str, value: &str) -> Result<(), DomainError> {
    if valid_id(value) {
        Ok(())
    } else {
        Err(DomainError::OutOfRange { field })
    }
}

pub fn refuse_evidence(value: &str) -> Result<(), DomainError> {
    if valid_digest(value) {
        Ok(())
    } else {
        Err(DomainError::OutOfRange {
            field: "evidence_digest",
        })
    }
}

pub fn refuse_date(field: &'static str, value: &str) -> Result<(), DomainError> {
    if valid_date(value) {
        Ok(())
    } else {
        Err(DomainError::Unsupported { field })
    }
}
