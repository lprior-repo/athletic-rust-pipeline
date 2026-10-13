use super::{key, EnvelopeInput, ProfileRegistryError, MAX_PROFILE_RECORD_BYTES};
use crate::{Store, StoreError, StoreResult};
use census_domain::model::{ProfileId, ProfileSelectionV1, QueryProfileV1};
use serde::Deserialize;

#[derive(Deserialize)]
struct VersionHeader {
    version: u32,
}

#[derive(Deserialize)]
struct PayloadHeader {
    payload: PayloadVersion,
}

#[derive(Deserialize)]
struct PayloadVersion {
    version: u32,
}

impl Store {
    pub fn profile(&self, id: &ProfileId) -> StoreResult<QueryProfileV1> {
        self.snapshot().profile(id)
    }

    pub fn selected_profile(&self, selection: &ProfileSelectionV1) -> StoreResult<QueryProfileV1> {
        self.snapshot().selected_profile(selection)
    }
}

pub(crate) fn retained(
    snapshot: &fjall::Snapshot,
    meta: &fjall::Keyspace,
    id: &ProfileId,
) -> StoreResult<Option<QueryProfileV1>> {
    use fjall::Readable;
    let record_key = key(id);
    let Some(length) = snapshot
        .size_of(meta, record_key.as_bytes())
        .map_err(|source| StoreError::Read { source })?
    else {
        return Ok(None);
    };
    let bytes = usize::try_from(length).map_err(|_| StoreError::CounterOverflow)?;
    if bytes > MAX_PROFILE_RECORD_BYTES {
        return Err(ProfileRegistryError::Oversized {
            bytes,
            max: MAX_PROFILE_RECORD_BYTES,
        }
        .into());
    }
    let raw = snapshot
        .get(meta, record_key.as_bytes())
        .map_err(|source| StoreError::Read { source })?
        .ok_or_else(|| ProfileRegistryError::MissingProfile { id: id.clone() })?;
    decode(id, raw.as_ref()).map(Some)
}

pub(crate) fn decode(id: &ProfileId, bytes: &[u8]) -> StoreResult<QueryProfileV1> {
    if bytes.len() > MAX_PROFILE_RECORD_BYTES {
        return Err(ProfileRegistryError::Oversized {
            bytes: bytes.len(),
            max: MAX_PROFILE_RECORD_BYTES,
        }
        .into());
    }
    let header: VersionHeader = parse(id, bytes)?;
    version("profile envelope", header.version)?;
    let payload_header: PayloadHeader = parse(id, bytes)?;
    version("query profile payload", payload_header.payload.version)?;
    let envelope: EnvelopeInput = parse(id, bytes)?;
    version("profile envelope", envelope.version)?;
    let profile =
        QueryProfileV1::new(envelope.payload).map_err(ProfileRegistryError::InvalidPayload)?;
    let computed = profile.id().map_err(ProfileRegistryError::Canonical)?;
    if &envelope.id != id || &computed != id {
        return Err(ProfileRegistryError::IdMismatch {
            requested: id.clone(),
            claimed: envelope.id,
            computed,
        }
        .into());
    }
    Ok(profile)
}

fn version(what: &'static str, found: u32) -> StoreResult<()> {
    if found != 1 {
        return Err(ProfileRegistryError::UnsupportedVersion { what, found }.into());
    }
    Ok(())
}

fn parse<'de, T: Deserialize<'de>>(id: &ProfileId, bytes: &'de [u8]) -> StoreResult<T> {
    serde_json::from_slice(bytes).map_err(|source| {
        ProfileRegistryError::Malformed {
            id: id.clone(),
            source,
        }
        .into()
    })
}
