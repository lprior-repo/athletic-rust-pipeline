use census_domain::model::StoreIdentity;
use census_domain::public_identity_history::{
    CandidateHistorySeed, DecodedCandidateHistory, RegistryHead, ReplayLimits,
};
use fjall::{Keyspace, Readable, Snapshot};
use serde::{de::DeserializeOwned, de::DeserializeSeed, Deserialize, Serialize};

use super::{PublicIdentityError, MAX_PUBLIC_IDENTITY_HISTORY_BYTES};
use crate::{StoreError, StoreResult};

pub(super) fn load(
    snapshot: &Snapshot,
    table: &Keyspace,
    key: &str,
    max: usize,
) -> StoreResult<Option<fjall::UserValue>> {
    let Some(size) = snapshot
        .size_of(table, key)
        .map_err(|source| StoreError::Read { source })?
    else {
        return Ok(None);
    };
    let bytes = usize::try_from(size).map_err(|_| StoreError::CounterOverflow)?;
    if bytes > max {
        return Err(StoreError::RowTooLarge { bytes, max });
    }
    let raw = snapshot
        .get(table, key)
        .map_err(|source| StoreError::Read { source })?
        .ok_or(PublicIdentityError::Corrupt("sized value disappeared"))?;
    if raw.len() > max {
        return Err(StoreError::RowTooLarge {
            bytes: raw.len(),
            max,
        });
    }
    Ok(Some(raw))
}

pub(super) fn decode<T: DeserializeOwned>(key: &str, raw: &[u8]) -> StoreResult<T> {
    serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
        key: key.to_owned(),
        source,
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct HeadInput {
    sequence: u64,
    digest: String,
}

pub(super) fn head(raw: &[u8]) -> StoreResult<RegistryHead> {
    let input: HeadInput = decode(super::HEAD, raw)?;
    Ok(RegistryHead {
        sequence: input.sequence,
        digest: input.digest,
    })
}

pub(super) fn history(
    raw: &[u8],
    identity: &StoreIdentity,
    head: &RegistryHead,
) -> StoreResult<DecodedCandidateHistory> {
    if raw.len() > MAX_PUBLIC_IDENTITY_HISTORY_BYTES {
        return Err(StoreError::RowTooLarge {
            bytes: raw.len(),
            max: MAX_PUBLIC_IDENTITY_HISTORY_BYTES,
        });
    }
    let mut parser = serde_json::Deserializer::from_slice(raw);
    let history = CandidateHistorySeed::new(identity, head, ReplayLimits::SMALL)
        .deserialize(&mut parser)
        .map_err(|source| StoreError::Decode {
            key: super::HISTORY.to_owned(),
            source,
        })?;
    parser.end().map_err(|source| StoreError::Decode {
        key: super::HISTORY.to_owned(),
        source,
    })?;
    Ok(history)
}

pub(super) fn encode<T: Serialize + ?Sized>(value: &T, max: usize) -> StoreResult<Vec<u8>> {
    let mut writer = Limited {
        bytes: Vec::new(),
        max,
    };
    serde_json::to_writer(&mut writer, value).map_err(|source| StoreError::Json {
        detail: "encoding bounded public identity state".to_owned(),
        source,
    })?;
    Ok(writer.bytes)
}

struct Limited {
    bytes: Vec<u8>,
    max: usize,
}

impl std::io::Write for Limited {
    fn write(&mut self, raw: &[u8]) -> std::io::Result<usize> {
        let needed = self
            .bytes
            .len()
            .checked_add(raw.len())
            .ok_or_else(|| std::io::Error::other("public identity byte overflow"))?;
        if needed > self.max {
            return Err(std::io::Error::other("public identity byte ceiling"));
        }
        if needed > self.bytes.capacity() {
            let doubled = self
                .bytes
                .capacity()
                .checked_mul(2)
                .map_or(self.max, |value| value.min(self.max));
            let target = doubled.max(256).max(needed).min(self.max);
            let additional = target
                .checked_sub(self.bytes.len())
                .ok_or_else(|| std::io::Error::other("public identity reserve accounting"))?;
            self.bytes
                .try_reserve_exact(additional)
                .map_err(std::io::Error::other)?;
        }
        self.bytes.extend_from_slice(raw);
        Ok(raw.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
