use super::{SchoolContactError, SourceRef};

pub(super) fn capture(
    source: &SourceRef,
    digest: &str,
    at: &str,
    statement: &str,
) -> Result<chrono::DateTime<chrono::FixedOffset>, SchoolContactError> {
    let url = source
        .url
        .as_deref()
        .ok_or(SchoolContactError::InvalidCapture)?;
    if source.id.trim().is_empty()
        || source.id.len() > 256
        || !locator_valid(url)
        || source.id.chars().any(char::is_control)
        || !digest_valid(digest)
        || at.len() > 64
        || statement.trim().is_empty()
        || statement.len() > 512
        || statement.chars().any(char::is_control)
    {
        return Err(SchoolContactError::InvalidCapture);
    }
    chrono::DateTime::parse_from_rfc3339(at).map_err(|_| SchoolContactError::InvalidCapture)
}

pub(super) fn attempt(attempt: &super::ContactResearchAttempt) -> bool {
    let digest_valid = attempt.source_sha256.as_deref().is_none_or(digest_valid);
    digest_valid
        && (!attempt.outcome.is_terminal()
            || attempt.source_sha256.is_some() && locator_valid(&attempt.locator))
        && attempt.acquired_at.len() <= 64
}

fn locator_valid(locator: &str) -> bool {
    if locator.len() > 4096 || locator.chars().any(char::is_whitespace) {
        return false;
    }
    let Ok(parsed) = url::Url::parse(locator) else {
        return false;
    };
    parsed.host_str().is_some()
        && matches!(parsed.scheme(), "http" | "https")
        && parsed.username().is_empty()
        && parsed.password().is_none()
}

fn digest_valid(digest: &str) -> bool {
    digest.len() == 64
        && digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
