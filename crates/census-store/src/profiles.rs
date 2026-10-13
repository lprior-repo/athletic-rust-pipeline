use census_domain::model::{CanonicalJsonError, ProfileError, ProfileId, ProfileSelectionError};
use serde::{Deserialize, Serialize};

pub(crate) mod read;
mod registration;

#[cfg(test)]
mod tests;

pub const MAX_PROFILE_RECORD_BYTES: usize = crate::records::codec::MAX_RECORD_BYTES;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileRegistration {
    Registered(ProfileId),
    Repeated(ProfileId),
}

#[derive(Debug, thiserror::Error)]
pub enum ProfileRegistryError {
    #[error("query profile {id} is missing")]
    MissingProfile { id: ProfileId },
    #[error("query profile record holds {bytes} bytes, past the {max}-byte ceiling")]
    Oversized { bytes: usize, max: usize },
    #[error("query profile {id} has malformed JSON: {source}")]
    Malformed {
        id: ProfileId,
        #[source]
        source: serde_json::Error,
    },
    #[error("unsupported {what} version {found}")]
    UnsupportedVersion { what: &'static str, found: u32 },
    #[error("query profile {requested} claims {claimed}, while its payload hashes to {computed}")]
    IdMismatch {
        requested: ProfileId,
        claimed: ProfileId,
        computed: ProfileId,
    },
    #[error("query profile {id} conflicts with its immutable payload")]
    ImmutableConflict { id: ProfileId },
    #[error(transparent)]
    InvalidPayload(#[from] ProfileError),
    #[error(transparent)]
    Selection(#[from] ProfileSelectionError),
    #[error(transparent)]
    Canonical(#[from] CanonicalJsonError),
}

#[derive(Serialize)]
struct Envelope<'a> {
    version: u32,
    id: &'a ProfileId,
    payload: &'a census_domain::model::QueryProfileV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvelopeInput {
    version: u32,
    id: ProfileId,
    payload: census_domain::model::QueryProfileInputV1,
}

pub(crate) fn key(id: &ProfileId) -> String {
    format!("profiles:v1:{id}")
}
