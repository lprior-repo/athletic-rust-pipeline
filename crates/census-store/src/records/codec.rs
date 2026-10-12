use super::model::Record;
use crate::{Store, StoreError, StoreResult};
use serde::{de::DeserializeOwned, Serialize};
use std::io::{self, Write};

pub(super) const MAX_RECORD_BYTES: usize = 65_536;
pub(super) const MAX_INVENTORY: usize = 4096;

pub(super) fn check_digest(digest: &str) -> StoreResult<()> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(StoreError::Refused {
            detail: "record digest must be lowercase sha256".to_string(),
        });
    }
    Ok(())
}
pub(super) fn check_inventory(inventory: &[String]) -> StoreResult<()> {
    if inventory.len() > MAX_INVENTORY {
        return Err(StoreError::RowTooLarge {
            bytes: inventory.len(),
            max: MAX_INVENTORY,
        });
    }
    for digest in inventory {
        check_digest(digest)?;
    }
    Ok(())
}
struct Size {
    bytes: usize,
}
impl Write for Size {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.bytes = self
            .bytes
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("record size overflow"))?;
        if self.bytes > MAX_RECORD_BYTES {
            return Err(io::Error::other("record byte ceiling"));
        }
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn encoded_size<T: Serialize + ?Sized>(record: &T) -> StoreResult<usize> {
    let mut size = Size { bytes: 0 };
    let result = serde_json::to_writer(&mut size, record);
    if size.bytes > MAX_RECORD_BYTES {
        return Err(StoreError::RowTooLarge {
            bytes: size.bytes,
            max: MAX_RECORD_BYTES,
        });
    }
    result.map_err(|source| StoreError::Json {
        detail: "measuring record".to_string(),
        source,
    })?;
    Ok(size.bytes)
}
pub(super) fn encode<T: Serialize + ?Sized>(record: &T) -> StoreResult<Vec<u8>> {
    let size = encoded_size(record)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(size)
        .map_err(|error| StoreError::Refused {
            detail: format!("record allocation: {error}"),
        })?;
    serde_json::to_writer(&mut bytes, record).map_err(|source| StoreError::Json {
        detail: "encoding record".to_string(),
        source,
    })?;
    Ok(bytes)
}
pub(super) fn decode<T: DeserializeOwned>(key: &str, bytes: &[u8]) -> StoreResult<T> {
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(StoreError::RowTooLarge {
            bytes: bytes.len(),
            max: MAX_RECORD_BYTES,
        });
    }
    serde_json::from_slice(bytes).map_err(|source| StoreError::Decode {
        key: key.to_string(),
        source,
    })
}
pub(super) fn key<T: Record>(digest: &str) -> String {
    format!("records/{}/{digest}", T::KIND)
}
pub(super) fn request_key<T: Record>(digest: &str) -> String {
    format!("record-requests/{}/{digest}", T::KIND)
}

impl Store {
    pub(super) fn record_read<T: Record>(&self, digest: &str) -> StoreResult<Option<T>> {
        check_digest(digest)?;
        let key = key::<T>(digest);
        let Some(bytes) = self
            .meta
            .get(key.as_bytes())
            .map_err(|source| StoreError::Read { source })?
        else {
            return Ok(None);
        };
        let mut record: T = decode(&key, &bytes)?;
        if let Some(inventory) = record.inventory_mut() {
            self.inventory_read(inventory)?;
        }
        record.validate()?;
        if record.digest() != digest {
            return Err(StoreError::Invariant {
                detail: format!("record key {key} disagrees with its digest"),
            });
        }
        Ok(Some(record))
    }
    pub(super) fn record_put<T: Record>(&self, record: &T) -> StoreResult<()> {
        record.validate()?;
        if !record.initial() {
            return Err(StoreError::Refused {
                detail: "record registration requires its initial revision and state".to_string(),
            });
        }
        let bytes = encode(record)?;
        let _appends = self.lock_appends();
        let request = request_key::<T>(record.digest());
        if let Some(retained) = self
            .meta
            .get(request.as_bytes())
            .map_err(|source| StoreError::Read { source })?
        {
            if retained.as_ref() != bytes.as_slice() {
                return Err(StoreError::RecordConflict {
                    kind: T::KIND,
                    digest: record.digest().to_string(),
                });
            }
            self.record_read::<T>(record.digest())?
                .ok_or_else(|| StoreError::RecordMissing {
                    kind: T::KIND,
                    digest: record.digest().to_string(),
                })?;
            return Ok(());
        }
        if self.record_read::<T>(record.digest())?.is_some() {
            return Err(StoreError::Invariant {
                detail: "record has no immutable request".to_string(),
            });
        }
        let mut batch = self.db.batch();
        if let Some(inventory) = record.inventory() {
            self.inventory_write(inventory, &mut batch)?;
        }
        batch.insert(&self.meta, request.as_bytes(), bytes.as_slice());
        batch.insert(&self.meta, key::<T>(record.digest()).as_bytes(), bytes);
        batch
            .durability(Some(fjall::PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })
    }
}
