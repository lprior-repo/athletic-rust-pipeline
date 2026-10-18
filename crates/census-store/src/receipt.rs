use serde::{Deserialize, Serialize};

use super::clock::{Clock, SystemClock};
use super::{Store, StoreError, StoreResult};

mod credit;
mod frontier;

pub(crate) mod scan;

pub const MAX_OPERATION_BYTES: usize = 512;

pub const MAX_DIGEST_BYTES: usize = 256;

pub const MAX_FRONTIER_WITNESSES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub operation: String,
    pub digest: String,
    pub at: String,
    pub appended: u64,
    #[serde(default)]
    pub credited: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayWitness {
    pub invocation: String,
    pub operation: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReplayFrontier {
    pub at: String,
    #[serde(default)]
    pub witnesses: Vec<ReplayWitness>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Credit {
    pub credited: u64,
    pub total: u64,
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
    crate::public_identity::refuse_reserved(operation)?;
    crate::evidence_subject_attachment::refuse_generic_operation(operation)?;
    crate::roster_source_issuance::refuse_generic_operation(operation)?;
    crate::result_school_correspondence::refuse_generic_operation(operation)?;
    if operation.starts_with("source_acquisition_attempt_") {
        return Err(crate::CaptureError::CaptureQualificationOptInRequired.into());
    }
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
    first_on(operation, digest, appended, || SystemClock.today())
}

pub(super) fn first_at(
    operation: &str,
    digest: &str,
    appended: u64,
    at: chrono::DateTime<chrono::Utc>,
) -> Receipt {
    first_on(operation, digest, appended, || at.date_naive().to_string())
}

fn first_on(operation: &str, digest: &str, appended: u64, at: impl FnOnce() -> String) -> Receipt {
    Receipt {
        operation: operation.to_string(),
        digest: digest.to_string(),
        at: at(),
        appended,
        credited: false,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pruned {
    pub removed: u64,
    pub undated: u64,
    #[serde(default)]
    pub witnesses_kept: u64,
    #[serde(default)]
    pub witnesses_missing: u64,
    #[serde(default)]
    pub frontier_advanced: bool,
}
