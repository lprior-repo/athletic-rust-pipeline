use serde::{Deserialize, Serialize};

use super::FetchError;

#[cfg(test)]
mod tests;

pub const MAX_REPRESENTATION_HEADERS: usize = 8;
pub const MAX_REPRESENTATION_NAME_BYTES: usize = 64;
pub const MAX_REPRESENTATION_VALUE_BYTES: usize = 1024;

const ALLOWED_NAMES: [&str; 3] = ["accept", "accept-language", "anettokens"];

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RepresentationHeaders {
    entries: Vec<(String, String)>,
}

impl RepresentationHeaders {
    pub fn canonical(source: &[(String, String)]) -> Result<Self, FetchError> {
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
            if !ALLOWED_NAMES.contains(&normalized.as_str()) {
                return Err(FetchError::Policy {
                    detail: format!(
                        "browser representation cannot carry the caller header {name}; allowed names are {ALLOWED_NAMES:?}"
                    ),
                });
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
            entries.push((normalized, value.clone()));
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
