use crate::{ProfileRegistryError, StoreResult};
use census_domain::model::{ProfileError, ProfileId};

pub(super) enum Entry {
    Profile(ProfileId),
    Unsupported,
    Malformed,
}

pub(super) fn supported_namespace(raw: &[u8]) -> bool {
    raw.starts_with(b"profiles:v1:")
}

pub(super) fn parse(raw: &[u8]) -> StoreResult<Entry> {
    let Some(suffix) = raw.strip_prefix(b"profiles:") else {
        return Ok(Entry::Malformed);
    };
    let Some(digest) = suffix.strip_prefix(b"v1:") else {
        return Ok(Entry::Unsupported);
    };
    let Ok(digest) = std::str::from_utf8(digest) else {
        return Ok(Entry::Malformed);
    };
    match ProfileId::parse(digest) {
        Ok(id) => Ok(Entry::Profile(id)),
        Err(ProfileError::AllocationFailed) => {
            Err(ProfileRegistryError::InvalidPayload(ProfileError::AllocationFailed).into())
        }
        Err(_) => Ok(Entry::Malformed),
    }
}
