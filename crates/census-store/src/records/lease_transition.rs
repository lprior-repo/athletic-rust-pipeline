use super::{
    encode, expiry, lease_key, LeaseMode, LeaseRecord, Store, StoreError, StoreResult, Transitioned,
};
use crate::records::codec::{check_digest, key};
use crate::records::model::{BuildRecord, InputRecord, PublishRecord, Record, UnitRecord};

impl Store {
    pub fn lease_reserve(
        &self,
        expected: &LeaseRecord,
        mode: LeaseMode,
        bytes: u64,
        available: u64,
    ) -> StoreResult<Transitioned> {
        if bytes > available {
            return Err(StoreError::DiskBudgetExceeded {
                needed: bytes,
                available,
            });
        }
        let _appends = self.lock_appends();
        let digest = expected.object_root.as_str();
        let current = self
            .lease_read(digest)?
            .ok_or_else(|| StoreError::LeaseLost {
                digest: digest.into(),
                epoch: expected.epoch,
            })?;
        if current.epoch != expected.epoch {
            return Err(StoreError::LeaseLost {
                digest: digest.into(),
                epoch: expected.epoch,
            });
        }
        if current != *expected {
            return Err(StoreError::RecordRevision {
                kind: "lease",
                digest: digest.into(),
                expected: expected.revision,
                actual: current.revision,
            });
        }
        if expiry(&current.expires_at)? <= chrono::Utc::now() {
            return Err(StoreError::LeaseExpired {
                digest: digest.into(),
                epoch: current.epoch,
            });
        }
        let mut next = current;
        next.revision = next
            .revision
            .checked_add(1)
            .ok_or(StoreError::CounterExhausted)?;
        next.mode = mode;
        next.reserved_bytes = bytes;
        next.validate()?;
        let mut batch = self.db.batch();
        batch.insert(&self.meta, lease_key(digest).as_bytes(), encode(&next)?);
        batch
            .durability(Some(fjall::PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        Ok(Transitioned {
            revision: next.revision,
        })
    }
    fn bind_record_lease<T: Record>(
        &self,
        digest: &str,
        expected: u64,
        epoch: u64,
        batch: &mut fjall::OwnedWriteBatch,
    ) -> StoreResult<usize> {
        if self.record_read::<T>(digest)?.is_none() {
            return Ok(0);
        }
        let mut record = self.record_expected::<T>(digest, expected)?;
        *record.lease_mut() = Some(epoch);
        *record.revision_mut() = expected
            .checked_add(1)
            .ok_or(StoreError::CounterExhausted)?;
        batch.insert(&self.meta, key::<T>(digest).as_bytes(), encode(&record)?);
        Ok(1)
    }
    pub fn record_lease(
        &self,
        digest: &str,
        expected: u64,
        epoch: u64,
    ) -> StoreResult<Transitioned> {
        check_digest(digest)?;
        let _appends = self.lock_appends();
        self.lease_check(digest, epoch)?;
        let mut batch = self.db.batch();
        let counts = [
            self.bind_record_lease::<InputRecord>(digest, expected, epoch, &mut batch)?,
            self.bind_record_lease::<BuildRecord>(digest, expected, epoch, &mut batch)?,
            self.bind_record_lease::<UnitRecord>(digest, expected, epoch, &mut batch)?,
            self.bind_record_lease::<PublishRecord>(digest, expected, epoch, &mut batch)?,
        ];
        if counts.iter().all(|count| *count == 0) {
            return Err(StoreError::RecordMissing {
                kind: "any",
                digest: digest.into(),
            });
        }
        let revision = expected
            .checked_add(1)
            .ok_or(StoreError::CounterExhausted)?;
        batch
            .durability(Some(fjall::PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        Ok(Transitioned { revision })
    }
}
