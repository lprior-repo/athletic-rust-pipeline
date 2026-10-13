use crate::{ProfileRegistryError, StoreResult};
use census_domain::model::{ContractRefV1, ProfileError, ProfileId};

pub(super) enum Entry<'a> {
    Profile(ProfileId),
    Descriptor(ContractRefV1),
    Chunk { descriptor: &'a [u8] },
    Unsupported,
    Malformed,
}

pub(super) fn supported_namespace(raw: &[u8]) -> bool {
    raw.starts_with(b"profiles:v1:") || raw.starts_with(b"region-contracts:v1/")
}

pub(super) fn parse(raw: &[u8]) -> StoreResult<Entry<'_>> {
    if let Some(suffix) = raw.strip_prefix(b"profiles:") {
        return profile(suffix);
    }
    if let Some(suffix) = raw.strip_prefix(b"region-contracts:") {
        return region(raw, suffix);
    }
    Ok(Entry::Malformed)
}

fn profile(suffix: &[u8]) -> StoreResult<Entry<'_>> {
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

fn region<'a>(raw: &'a [u8], suffix: &[u8]) -> StoreResult<Entry<'a>> {
    if !suffix.starts_with(b"v1/") {
        return Ok(Entry::Unsupported);
    }
    let descriptor = match raw.iter().position(|byte| *byte == 1) {
        Some(position) => raw.get(..position).map_or(&[][..], core::convert::identity),
        None => raw,
    };
    let Some(reference) = reference(descriptor)? else {
        return Ok(Entry::Malformed);
    };
    if descriptor.len() == raw.len() {
        return Ok(Entry::Descriptor(reference));
    }
    let expected = descriptor.len().checked_add(5);
    if expected != Some(raw.len()) {
        return Ok(Entry::Malformed);
    }
    match crate::journal::parse_chunk_key(raw) {
        Some((logical, _)) if logical == descriptor => Ok(Entry::Chunk { descriptor }),
        _ => Ok(Entry::Malformed),
    }
}

fn reference(descriptor: &[u8]) -> StoreResult<Option<ContractRefV1>> {
    let Ok(text) = std::str::from_utf8(descriptor) else {
        return Ok(None);
    };
    let Some(suffix) = text.strip_prefix("region-contracts:v1/") else {
        return Ok(None);
    };
    let Some((revision, digest)) = suffix.split_once('/') else {
        return Ok(None);
    };
    let Ok(number) = revision.parse::<u32>() else {
        return Ok(None);
    };
    if number.to_string() != revision {
        return Ok(None);
    }
    match ContractRefV1::new(1, number, digest) {
        Ok(reference) => Ok(Some(reference)),
        Err(ProfileError::AllocationFailed) => {
            Err(ProfileRegistryError::InvalidPayload(ProfileError::AllocationFailed).into())
        }
        Err(_) => Ok(None),
    }
}
