use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::super::ledger;
use super::config::validate_operation;
use super::error::{artifact, io, BoundaryError};
use super::files::{Directory, MAX_BYTES};

pub(super) const BASENAME: &str = "teams-source-reservation-reached.json";
pub(super) const PENDING: &str = "teams-source-reservation-reached.json.pending";
const PHASE: &str = "teams_reserved_before_acquisition";

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(super) struct Marker {
    schema: u8,
    phase: String,
    operation: String,
    attempt: u8,
    request_digest: String,
    observed_on: String,
}

impl Marker {
    pub(super) fn new_for_kind(
        operation: &str,
        attempt: u8,
        kind: &str,
        identity: &ledger::Identity,
    ) -> Result<Self, BoundaryError> {
        validate_operation(kind)?;
        let mut marker = Self::new(operation, attempt, identity)?;
        marker.phase = kind.to_string();
        Ok(marker)
    }

    pub(super) fn new(
        operation: &str,
        attempt: u8,
        identity: &ledger::Identity,
    ) -> Result<Self, BoundaryError> {
        validate_operation(operation)?;
        if !(1..=3).contains(&attempt) {
            return Err(BoundaryError::Attempt(attempt));
        }
        if identity.request_digest.is_empty()
            || identity.request_digest.len() > 128
            || identity.observed_on.is_empty()
            || identity.observed_on.len() > 64
        {
            return Err(BoundaryError::IdentityBounds);
        }
        Ok(Self {
            schema: 1,
            phase: PHASE.to_string(),
            operation: operation.to_string(),
            attempt,
            request_digest: identity.request_digest.clone(),
            observed_on: identity.observed_on.clone(),
        })
    }

    pub(super) fn publish(&self, directory: &Directory) -> Result<(), BoundaryError> {
        let target = directory.child(BASENAME.as_ref());
        let pending = directory.child(PENDING.as_ref());
        if directory.inspect(&pending)?.is_some() {
            return Err(artifact(&pending, "unexpected preexisting pending file"));
        }
        if directory.inspect(&target)?.is_some() {
            let existing: Self = directory.read(&target)?;
            if existing != *self {
                return Err(BoundaryError::IdentityMismatch);
            }
            return sync_existing(directory, &target);
        }
        let bytes = serde_json::to_vec(self)?;
        if bytes.len() > MAX_BYTES {
            return Err(artifact(&target, "encoded marker exceeds 4096 bytes"));
        }
        let mut file = directory.create_pending(&pending)?;
        let publication = publish_pending(directory, &mut file, &pending, &target, &bytes);
        let cleanup = directory
            .remove_owned(&pending, &file)
            .and_then(|()| directory.sync());
        finish_publication(publication, cleanup)
    }
}

fn publish_pending(
    directory: &Directory,
    file: &mut File,
    pending: &Path,
    target: &Path,
    bytes: &[u8],
) -> Result<(), BoundaryError> {
    file.write_all(bytes)
        .map_err(|source| io("writing complete marker", pending, source))?;
    file.sync_all()
        .map_err(|source| io("syncing complete marker", pending, source))?;
    directory.verify_owned(pending, file)?;
    fs::hard_link(pending, target)
        .map_err(|source| io("publishing marker without overwrite", target, source))?;
    directory.verify_owned(target, file)?;
    directory.sync()
}

fn sync_existing(directory: &Directory, target: &Path) -> Result<(), BoundaryError> {
    directory.sync_file(target)?;
    directory.sync()
}

fn finish_publication(
    publication: Result<(), BoundaryError>,
    cleanup: Result<(), BoundaryError>,
) -> Result<(), BoundaryError> {
    match (publication, cleanup) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
        (Err(primary), Err(cleanup)) => Err(BoundaryError::Cleanup {
            primary: Box::new(primary),
            cleanup: Box::new(cleanup),
        }),
    }
}
