use super::shell;
use crate::net::cache::{CacheMeta, EligibleCacheV2};
use crate::net::FetchError;
use census_domain::model::AcquisitionOccurrenceRefV1;
use census_store::RecoveryArchiveIo;
use serde::Serialize;

mod canonical;
mod tree;
pub(super) mod writer;

pub(super) enum Compatible {
    NotReusable,
    Encoded {
        metadata: CacheMeta,
        envelope: Vec<u8>,
    },
    MetadataChanged,
    EnvelopeTooLarge,
}

pub(super) fn recovered(
    bytes: &[u8],
    occurrence: &AcquisitionOccurrenceRefV1,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<Compatible, FetchError> {
    let metadata = decode_metadata(bytes, io)?;
    if metadata.status != 200 {
        return Ok(Compatible::NotReusable);
    }
    let canonical = canonical::Metadata(&metadata);
    let Some(count) = writer::count(&canonical, io)? else {
        return Ok(Compatible::MetadataChanged);
    };
    let encoded = writer::encode(&canonical, count, io)?;
    if encoded != bytes {
        return Ok(Compatible::MetadataChanged);
    }
    io.release_owned(encoded.capacity())?;
    drop(encoded);
    let envelope = BorrowedEnvelope {
        version: 2,
        metadata: &metadata,
        original_occurrence: occurrence,
    };
    let Some(count) = writer::count(&envelope, io)? else {
        return Ok(Compatible::EnvelopeTooLarge);
    };
    let envelope = writer::encode(&envelope, count, io)?;
    Ok(Compatible::Encoded { metadata, envelope })
}

#[derive(Serialize)]
struct BorrowedEnvelope<'a> {
    version: u8,
    metadata: &'a CacheMeta,
    original_occurrence: &'a AcquisitionOccurrenceRefV1,
}

pub(super) fn canonical(
    metadata: &CacheMeta,
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<Vec<u8>, FetchError> {
    let value = canonical::Metadata(metadata);
    let count =
        writer::count(&value, io)?.ok_or_else(|| shell::limit("canonical metadata encoding"))?;
    writer::encode(&value, count, io)
}

pub(super) fn decode_metadata(
    bytes: &[u8],
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<CacheMeta, FetchError> {
    decode_admission(bytes.len(), io)?;
    match serde_json::from_slice(bytes) {
        Ok(metadata) => Ok(metadata),
        Err(source) => {
            super::diagnostic::error(&source, 512, io)?;
            Err(FetchError::Decode {
                target: "prepared metadata".to_string(),
                source,
            })
        }
    }
}

pub(super) fn decode_previous(
    bytes: &[u8],
    io: &mut RecoveryArchiveIo<'_>,
) -> Result<(CacheMeta, Vec<u8>), FetchError> {
    decode_admission(bytes.len(), io)?;
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let envelope = match serde::de::Deserializer::deserialize_map(&mut decoder, Probe) {
        Ok(envelope) => envelope,
        Err(source) => {
            super::diagnostic::error(&source, 512, io)?;
            return Err(FetchError::Decode {
                target: "previous cache metadata".to_string(),
                source,
            });
        }
    };
    if let Err(source) = decoder.end() {
        super::diagnostic::error(&source, 512, io)?;
        return Err(FetchError::Decode {
            target: "previous cache metadata".to_string(),
            source,
        });
    }
    if envelope {
        let envelope: EligibleCacheV2 = serde_json::from_slice(bytes)
            .map_err(|_| crate::net::CaptureTransportError::CacheEnvelopeUnsupported)?;
        let canonical = canonical(&envelope.metadata, io)?;
        Ok((envelope.metadata, canonical))
    } else {
        let canonical = tree::canonical(bytes, io)?;
        let metadata = decode_metadata(&canonical, io)?;
        Ok((metadata, canonical))
    }
}

struct Probe;

impl<'de> serde::de::Visitor<'de> for Probe {
    type Value = bool;
    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("cache metadata object")
    }
    fn visit_map<A: serde::de::MapAccess<'de>>(self, mut access: A) -> Result<bool, A::Error> {
        let mut envelope = false;
        while let Some(name) = access.next_key::<String>()? {
            if matches!(
                name.as_str(),
                "version" | "metadata" | "original_occurrence"
            ) {
                envelope = true;
            }
            access.next_value::<serde::de::IgnoredAny>()?;
        }
        Ok(envelope)
    }
}

fn decode_admission(bytes: usize, io: &mut RecoveryArchiveIo<'_>) -> Result<(), FetchError> {
    if bytes > super::super::MAX_META_BYTES {
        return Err(shell::limit("metadata length"));
    }
    let bound = bytes
        .checked_mul(192)
        .and_then(|count| count.checked_add(4096))
        .ok_or_else(|| shell::limit("allocation"))?;
    io.reserve_owned(bound)?;
    Ok(())
}
