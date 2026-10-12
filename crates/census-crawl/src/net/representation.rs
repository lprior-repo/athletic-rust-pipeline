use serde::{Deserialize, Serialize};

use super::FetchError;

mod source_policy;
#[cfg(test)]
mod tests;

pub const MAX_REPRESENTATION_HEADERS: usize = 8;
pub const MAX_REPRESENTATION_NAME_BYTES: usize = 64;
pub const MAX_REPRESENTATION_VALUE_BYTES: usize = 1024;
pub const MAX_USER_AGENT_BYTES: usize = 200;

pub const USER_AGENT_NAME: &str = "user-agent";

const ALLOWED_NAMES: [&str; 4] = ["accept", "accept-language", "anettokens", USER_AGENT_NAME];

pub fn validate_user_agent(value: &str) -> Result<(), FetchError> {
    let reason = if value.is_empty() {
        "it is empty, so it declares no agent"
    } else if value.len() > MAX_USER_AGENT_BYTES {
        "it exceeds the compact agent byte ceiling"
    } else if value.contains('\r') || value.contains('\n') {
        "it spans more than one line"
    } else if value.bytes().any(|byte| !matches!(byte, b' '..=b'~')) {
        "it carries a byte outside printable ASCII"
    } else if value.contains('(') || value.contains(')') {
        "it carries a parenthesized comment form, so it is not a compact agent"
    } else if value.trim() != value {
        "it carries leading or trailing whitespace"
    } else {
        return Ok(());
    };
    Err(FetchError::Policy {
        detail: format!(
            "representation User-Agent {value:?} is refused: {reason}; declare a compact agent of at most {MAX_USER_AGENT_BYTES} printable ASCII bytes"
        ),
    })
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RepresentationHeaders {
    entries: Vec<(String, String)>,
}

impl RepresentationHeaders {
    pub fn canonical(source: &[(String, String)]) -> Result<Self, FetchError> {
        Self::canonical_for(None, source)
    }

    pub(crate) fn for_request(url: &str, source: &[(String, String)]) -> Result<Self, FetchError> {
        Self::canonical_for(Some(url), source)
    }

    fn canonical_for(url: Option<&str>, source: &[(String, String)]) -> Result<Self, FetchError> {
        if source.len() > MAX_REPRESENTATION_HEADERS {
            return Err(FetchError::Policy {
                detail: format!(
                    "browser representation declares {} headers, the ceiling is {MAX_REPRESENTATION_HEADERS}",
                    source.len()
                ),
            });
        }
        let mut entries: Vec<(String, String)> = Vec::with_capacity(source.len());
        for (name, value) in source {
            let normalized = name.trim().to_ascii_lowercase();
            if !ALLOWED_NAMES.contains(&normalized.as_str())
                && !url.is_some_and(|url| source_policy::permits(url, &normalized, value))
            {
                return Err(FetchError::Policy {
                    detail: format!(
                        "browser representation cannot carry the caller header {name}; allowed names are {ALLOWED_NAMES:?}"
                    ),
                });
            }
            if normalized == USER_AGENT_NAME {
                validate_user_agent(value)?;
            }
            if normalized.len() > MAX_REPRESENTATION_NAME_BYTES
                || value.len() > MAX_REPRESENTATION_VALUE_BYTES
            {
                return Err(FetchError::Policy {
                    detail: format!(
                        "browser representation header {normalized} exceeds the bounded name or value size"
                    ),
                });
            }
            if entries.iter().any(|(existing, _)| *existing == normalized) {
                return Err(FetchError::Policy {
                    detail: format!("browser representation repeats the header {normalized}"),
                });
            }
            if reqwest::header::HeaderValue::from_str(value).is_err() {
                return Err(FetchError::Policy {
                    detail: format!("representation header {normalized} is not an HTTP value"),
                });
            }
            let recorded = source_policy::recorded_value(&normalized, value);
            entries.push((normalized, recorded));
        }
        entries.sort();
        Ok(Self { entries })
    }

    pub fn from_entries(entries: Vec<(String, String)>) -> Result<Self, FetchError> {
        Self::canonical(&entries)
    }

    pub fn entries(&self) -> &[(String, String)] {
        &self.entries
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn identity(&self) -> String {
        let mut identity = String::new();
        for (name, value) in &self.entries {
            identity.push_str(name);
            identity.push('=');
            identity.push_str(value);
            identity.push('\u{1f}');
        }
        identity
    }
}
