use std::path::Path;
use std::time::Duration;

use serde::Deserialize;

use super::error::BoundaryError;
use super::files::Directory;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct RawConfig {
    schema: u8,
    operation: String,
    attempt: u8,
    timeout_seconds: u64,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    chunk_index: Option<u32>,
}

pub(super) struct Config {
    pub(super) operation: String,
    pub(super) attempt: u8,
    timeout: Duration,
    kind: Option<String>,
    chunk_index: Option<u32>,
}

impl Config {
    pub(super) fn read(directory: &Directory, path: &Path) -> Result<Self, BoundaryError> {
        let raw: RawConfig = directory.read(path)?;
        Self::parse(raw)
    }

    pub(super) fn parse(raw: RawConfig) -> Result<Self, BoundaryError> {
        if raw.schema == 1 {
            if raw.kind.is_some() || raw.chunk_index.is_some() {
                return Err(BoundaryError::Schema(1));
            }
        } else if raw.schema == 2 {
            if raw.kind.is_none() {
                return Err(BoundaryError::Schema(2));
            }
        } else {
            return Err(BoundaryError::Schema(raw.schema));
        }
        validate_operation(&raw.operation)?;
        if !(1..=3).contains(&raw.attempt) {
            return Err(BoundaryError::Attempt(raw.attempt));
        }
        if !(1..=60).contains(&raw.timeout_seconds) {
            return Err(BoundaryError::TimeoutBounds(raw.timeout_seconds));
        }
        if let Some(kind) = &raw.kind {
            validate_operation(kind)?;
        }
        Ok(Self {
            operation: raw.operation,
            attempt: raw.attempt,
            timeout: Duration::from_secs(raw.timeout_seconds),
            kind: raw.kind,
            chunk_index: raw.chunk_index,
        })
    }

    pub(super) fn select(&self, operation: &str, attempt: u8) -> Selection {
        let reservation = self.kind.as_deref() == Some("teams_reserved_before_acquisition")
            || self.kind.is_none();
        if reservation && self.operation == operation && self.attempt == attempt {
            Selection::Hold(self.timeout)
        } else {
            Selection::Continue
        }
    }

    pub(super) fn select_point(
        &self,
        point: &census_crawl::milesplit::boundary::Point,
    ) -> Selection {
        let Some(kind) = &self.kind else {
            return Selection::Continue;
        };
        if point.kind() != kind {
            return Selection::Continue;
        }
        match self.chunk_index {
            None => Selection::Hold(self.timeout),
            Some(chunk) => match point {
                census_crawl::milesplit::boundary::Point::PageChunk { index }
                    if *index == chunk =>
                {
                    Selection::Hold(self.timeout)
                }
                _ => Selection::Continue,
            },
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
