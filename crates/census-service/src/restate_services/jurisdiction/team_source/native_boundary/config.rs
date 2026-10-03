use std::path::Path;
use std::time::Duration;

use serde::Deserialize;

use super::error::BoundaryError;
use super::files::Directory;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    schema: u8,
    operation: String,
    attempt: u8,
    timeout_seconds: u64,
}

pub(super) struct Config {
    operation: String,
    attempt: u8,
    timeout: Duration,
}

impl Config {
    pub(super) fn read(directory: &Directory, path: &Path) -> Result<Self, BoundaryError> {
        let raw: RawConfig = directory.read(path)?;
        Self::parse(raw)
    }

    fn parse(raw: RawConfig) -> Result<Self, BoundaryError> {
        if raw.schema != 1 {
            return Err(BoundaryError::Schema(raw.schema));
        }
        validate_operation(&raw.operation)?;
        if !(1..=3).contains(&raw.attempt) {
            return Err(BoundaryError::Attempt(raw.attempt));
        }
        if !(1..=60).contains(&raw.timeout_seconds) {
            return Err(BoundaryError::TimeoutBounds(raw.timeout_seconds));
        }
        Ok(Self {
            operation: raw.operation,
            attempt: raw.attempt,
            timeout: Duration::from_secs(raw.timeout_seconds),
        })
    }

    pub(super) fn select(&self, operation: &str, attempt: u8) -> Selection {
        if self.operation == operation && self.attempt == attempt {
            Selection::Hold(self.timeout)
        } else {
            Selection::Continue
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Selection {
    Continue,
    Hold(Duration),
}

pub(super) fn validate_operation(operation: &str) -> Result<(), BoundaryError> {
    if operation.is_empty()
        || operation.len() > 1024
        || operation.chars().any(|character| {
            character.is_control()
                || character.is_whitespace()
                || character == '*'
                || character == '?'
        })
    {
        return Err(BoundaryError::Operation);
    }
    Ok(())
}
