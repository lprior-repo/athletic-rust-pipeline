use crate::domain::error::DomainError;

pub(super) fn validate_text(
    raw: &str,
    field: &'static str,
    limit: usize,
) -> Result<(), DomainError> {
    if raw.trim().is_empty() {
        return Err(DomainError::Empty { field });
    }
    if raw.len() > limit {
        return Err(DomainError::TooLong { field, limit });
    }
    if raw.chars().any(char::is_control) {
        return Err(DomainError::InvalidFormat { field });
    }
    Ok(())
}
