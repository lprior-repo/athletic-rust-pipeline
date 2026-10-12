use super::codec::{check_inventory, decode};
use super::model::{BuildRecord, BuildState, InputRecord, InputState, UnitRecord, UnitState};
use super::publish::Digest;
use super::transition::state;
use crate::{Store, StoreError, StoreResult};
use sha2::{Digest as _, Sha256};

pub(super) fn output_key(output: &Digest) -> String {
    format!("records/output/{}", output.as_str())
}
pub(super) fn receipt_key(digest: &str) -> String {
    format!("records/unit/{digest}")
}
impl Store {
    pub(super) fn unit_output_complete(&self, record: &UnitRecord) -> StoreResult<bool> {
        let UnitState::Complete { output } = &record.state else {
            return Ok(false);
        };
        let bytes = self
            .meta
            .get(output_key(output).as_bytes())
            .map_err(|source| StoreError::Read { source })?
            .ok_or_else(|| StoreError::CorruptArtifact {
                digest: output.as_str().into(),
            })?;
        let key = receipt_key(record.digest());
        let raw = self
            .receipts
            .get(key.as_bytes())
            .map_err(|source| StoreError::Read { source })?
            .ok_or_else(|| StoreError::CorruptArtifact {
                digest: output.as_str().into(),
            })?;
        let receipt: crate::Receipt = decode(&key, &raw)?;
        if bytes.len() > super::codec::MAX_RECORD_BYTES
            || format!("{:x}", Sha256::digest(&bytes)) != output.as_str()
            || receipt.operation != key
            || receipt.digest != output.as_str()
            || receipt.appended != 0
            || receipt.credited
        {
            return Err(StoreError::CorruptArtifact {
                digest: output.as_str().into(),
            });
        }
        Ok(true)
    }
    pub(super) fn unit_build_running(&self, record: &UnitRecord) -> StoreResult<()> {
        let build = self
            .record_read::<BuildRecord>(record.build_digest())?
            .ok_or_else(|| StoreError::RecordMissing {
                kind: "build",
                digest: record.build_digest().into(),
            })?;
        state(&build, build.state, BuildState::Building)?;
        if build.input_digest != record.input_digest
            || !build
                .required_units()
                .iter()
                .any(|id| id == record.digest())
        {
            return Err(StoreError::RecordConflict {
                kind: "unit",
                digest: record.digest().into(),
            });
        }
        Ok(())
    }
    pub(super) fn build_inventory_complete(&self, build: &BuildRecord) -> StoreResult<()> {
        build.required_units().iter().try_for_each(|digest| {
            let record = self.record_read::<UnitRecord>(digest)?.ok_or_else(|| {
                StoreError::RecordMissing {
                    kind: "unit",
                    digest: digest.clone(),
                }
            })?;
            if record.build_digest != build.digest
                || record.input_digest != build.input_digest
                || !self.unit_output_complete(&record)?
            {
                return Err(StoreError::VerificationFailed {
                    digest: digest.clone(),
                });
            }
            Ok(())
        })
    }
    fn unit_chain_complete(&self, record: &UnitRecord) -> StoreResult<bool> {
        let input = self.record_read::<InputRecord>(record.input_digest())?;
        let build = self.record_read::<BuildRecord>(record.build_digest())?;
        Ok(self.unit_output_complete(record)?
            && input.is_some_and(|input| input.state == InputState::Frozen)
            && build.is_some_and(|build| {
                matches!(
                    build.state,
                    BuildState::Building
                        | BuildState::Built
                        | BuildState::Verifying
                        | BuildState::Verified
                ) && build.input_digest == record.input_digest
                    && build
                        .required_units()
                        .iter()
                        .any(|id| id == record.digest())
            }))
    }
    fn complete_unit(&self, digest: &str) -> StoreResult<bool> {
        match self.record_read::<UnitRecord>(digest)? {
            Some(record) => self.unit_chain_complete(&record),
            None => Ok(false),
        }
    }
    pub(super) fn publication_unit_verified(&self, digest: &str) -> StoreResult<()> {
        let record =
            self.record_read::<UnitRecord>(digest)?
                .ok_or_else(|| StoreError::RecordMissing {
                    kind: "unit",
                    digest: digest.into(),
                })?;
        if !self.unit_chain_complete(&record)? {
            return Err(StoreError::VerificationFailed {
                digest: digest.into(),
            });
        }
        let build = self
            .record_read::<BuildRecord>(record.build_digest())?
            .ok_or_else(|| StoreError::RecordMissing {
                kind: "build",
                digest: record.build_digest().into(),
            })?;
        state(&build, build.state, BuildState::Verified)
    }
    pub fn unit_replay_complete(&self, digest: &str, inventory: &[String]) -> StoreResult<bool> {
        check_inventory(inventory)?;
        let _appends = self.lock_appends();
        let record = self.record_read::<UnitRecord>(digest)?;
        let complete = match &record {
            Some(record) => self.unit_chain_complete(record)?,
            None => false,
        };
        inventory.iter().try_fold(complete, |complete, required| {
            let bound = record
                .as_ref()
                .is_some_and(|r| required == r.input_digest() || required == r.build_digest());
            let valid = if bound {
                true
            } else {
                self.complete_unit(required)?
            };
            Ok(complete && valid)
        })
    }
    pub fn get_unit_output(&self, digest: &str) -> StoreResult<Option<Vec<u8>>> {
        let _appends = self.lock_appends();
        let Some(record) = self.record_read::<UnitRecord>(digest)? else {
            return Ok(None);
        };
        if !self.unit_chain_complete(&record)? {
            return Ok(None);
        }
        let UnitState::Complete { output } = &record.state else {
            return Ok(None);
        };
        let bytes = self
            .meta
            .get(output_key(output).as_bytes())
            .map_err(|source| StoreError::Read { source })?
            .ok_or_else(|| StoreError::CorruptArtifact {
                digest: output.as_str().into(),
            })?;
        Ok(Some(bytes.to_vec()))
    }
    pub fn inventory_reconcile(&self, required: &[String]) -> StoreResult<Vec<String>> {
        check_inventory(required)?;
        let _appends = self.lock_appends();
        let mut missing = Vec::new();
        missing
            .try_reserve_exact(required.len())
            .map_err(|_| StoreError::BudgetExceeded {
                what: "inventory reconciliation allocation",
                needed: 4096,
                available: 0,
            })?;
        required.iter().try_fold(missing, |mut missing, digest| {
            if !self.complete_unit(digest)? {
                missing.push(digest.clone());
            }
            Ok(missing)
        })
    }
}
