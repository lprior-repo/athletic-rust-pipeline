
use chrono::NaiveDate;
use fjall::PersistMode;
use serde::{Deserialize, Serialize};

use super::clock::{Clock, SystemClock};
use super::{Store, StoreError, StoreResult};

pub const MAX_OPERATION_BYTES: usize = 512;

pub const MAX_DIGEST_BYTES: usize = 256;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub operation: String,
    pub digest: String,
    pub at: String,
    pub appended: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Application {
    Written(Receipt),
    Repeated(Receipt),
}

impl Application {
    pub fn appended(&self) -> u64 {
        match self {
            Self::Written(receipt) => receipt.appended,
            Self::Repeated(_) => 0,
        }
    }

    pub fn written(&self) -> bool {
        matches!(self, Self::Written(_))
    }

    pub fn repeated(&self) -> bool {
        matches!(self, Self::Repeated(_))
    }

    pub fn receipt(&self) -> &Receipt {
        match self {
            Self::Written(receipt) | Self::Repeated(receipt) => receipt,
        }
    }
}

pub(super) enum Decision {
    First,
    Repeat(Receipt),
}

impl Store {
    pub fn receipt(&self, operation: &str) -> StoreResult<Option<Receipt>> {
        let raw = self
            .receipts
            .get(operation.as_bytes())
            .map_err(|source| StoreError::Read { source })?;
        let Some(value) = raw else {
            return Ok(None);
        };
        let receipt: Receipt =
            serde_json::from_slice(value.as_ref()).map_err(|source| StoreError::Decode {
                key: format!("receipts/{operation}"),
                source,
            })?;
        Ok(Some(receipt))
    }

    pub fn receipt_count(&self) -> StoreResult<u64> {
        let count = self
            .receipts
            .len()
            .map_err(|source| StoreError::Read { source })?;
        u64::try_from(count).map_err(|_| StoreError::CounterOverflow)
    }
}

pub(super) fn refuse_over_operation(operation: &str, digest: &str) -> StoreResult<()> {
    if operation.is_empty() {
        return Err(StoreError::Refused {
            detail: "operation id must not be empty: a receipt keyed by nothing records nothing"
                .to_string(),
        });
    }
    if operation.len() > MAX_OPERATION_BYTES {
        return Err(StoreError::Refused {
            detail: format!(
                "operation id of {} bytes exceeds the {MAX_OPERATION_BYTES}-byte ceiling",
                operation.len()
            ),
        });
    }
    if digest.is_empty() {
        return Err(StoreError::Refused {
            detail: "payload digest must not be empty: an empty digest matches every payload"
                .to_string(),
        });
    }
    if digest.len() > MAX_DIGEST_BYTES {
        return Err(StoreError::Refused {
            detail: format!(
                "payload digest of {} bytes exceeds the {MAX_DIGEST_BYTES}-byte ceiling",
                digest.len()
            ),
        });
    }
    Ok(())
}

pub(super) fn decide(store: &Store, operation: &str, digest: &str) -> StoreResult<Decision> {
    match store.receipt(operation)? {
        None => Ok(Decision::First),
        Some(receipt) if receipt.digest == digest => Ok(Decision::Repeat(receipt)),
        Some(receipt) => Err(StoreError::Invariant {
            detail: format!(
                "operation {operation} was applied on {} with digest {} and is now offered digest \
                 {digest}: the same id cannot name two payloads",
                receipt.at, receipt.digest
            ),
        }),
    }
}

pub(super) fn first(operation: &str, digest: &str, appended: u64) -> Receipt {
    Receipt {
        operation: operation.to_string(),
        digest: digest.to_string(),
        at: SystemClock.today(),
        appended,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pruned {
    pub removed: u64,
    pub undated: u64,
}

impl Store {
    pub fn prune_receipts(&self, before: &str) -> StoreResult<Pruned> {
        let boundary =
            NaiveDate::parse_from_str(before, "%Y-%m-%d").map_err(|_| StoreError::Refused {
                detail: format!("retention boundary {before} is not a YYYY-MM-DD day"),
            })?;
        let _appends = self.lock_appends();
        let mut pruned = Pruned::default();
        let mut doomed: Vec<Vec<u8>> = Vec::new();
        for guard in self.receipts.iter() {
            let (key, raw) = guard
                .into_inner()
                .map_err(|source| StoreError::Read { source })?;
            let receipt: Receipt =
                serde_json::from_slice(raw.as_ref()).map_err(|source| StoreError::Decode {
                    key: format!("receipts/{}", String::from_utf8_lossy(&key)),
                    source,
                })?;
            match NaiveDate::parse_from_str(&receipt.at, "%Y-%m-%d") {
                Ok(stamped) if stamped < boundary => doomed.push(key.to_vec()),
                Ok(_) => {}
                Err(_) => pruned.undated = pruned.undated.saturating_add(1),
            }
        }
        if doomed.is_empty() {
            return Ok(pruned);
        }
        pruned.removed = u64::try_from(doomed.len()).map_err(|_| StoreError::CounterOverflow)?;
        let mut batch = self.db.batch();
        for key in doomed {
            batch.remove(&self.receipts, key);
        }
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        Ok(pruned)
    }
}
