use super::*;

pub(super) fn read_meta_digest(
    path: &Path,
    label: &str,
    issues: &mut Vec<String>,
) -> crate::StoreResult<Option<String>> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(_) => {
            issues.push(format!("meta {label}: unreadable"));
            return Ok(None);
        }
    };
    parse_meta_digest(&bytes, label, issues)
}

pub(super) fn parse_meta_digest(
    bytes: &[u8],
    label: &str,
    issues: &mut Vec<String>,
) -> crate::StoreResult<Option<String>> {
    let Ok(meta) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        issues.push(format!("meta {label}: invalid JSON"));
        return Ok(None);
    };
    if meta.get("version").is_some()
        || meta.get("metadata").is_some()
        || meta.get("original_occurrence").is_some()
    {
        return match crate::acquisition::cache::envelope_digest(bytes) {
            Ok(Some(digest)) => Ok(Some(digest)),
            Ok(None) => Err(crate::CaptureError::CacheEnvelopeUnsupported.into()),
            Err(error) => {
                issues.push(format!("meta {label}: {error}"));
                Ok(None)
            }
        };
    }
    match meta.get("content_digest").and_then(|value| value.as_str()) {
        Some(value) if is_digest(value) => Ok(Some(value.to_string())),
        _ => {
            issues.push(format!("meta {label}: missing or invalid content_digest"));
            Ok(None)
        }
    }
}
