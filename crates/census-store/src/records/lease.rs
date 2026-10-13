use super::codec::{check_digest, decode, encode, key};
use super::model::{InputRecord, LeaseMode, LeaseRecord, Transitioned};
use super::publish::Digest;
use crate::{meta, Store, StoreError, StoreResult};
use chrono::{DateTime, NaiveDate, Utc};
use sha2::{Digest as _, Sha256};

const MAX_DURABLE_LEASES: usize = 4096;

#[path = "lease_transition.rs"]
mod transition;
fn lease_key(digest: &str) -> String {
    format!("lease/{digest}")
}
fn expiry(value: &str) -> StoreResult<DateTime<Utc>> {
    if let Ok(date) = DateTime::parse_from_rfc3339(value) {
        return Ok(date.with_timezone(&Utc));
    }
    let end = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.succ_opt())
        .and_then(|date| date.and_hms_opt(0, 0, 0));
    end.map(|date| date.and_utc())
        .ok_or_else(|| StoreError::Refused {
            detail: "lease expiry must be RFC3339 or a supported YYYY-MM-DD date".into(),
        })
}
impl LeaseRecord {
    pub fn new(owner: &str, expires_at: &str, epoch: u64, object_root: &str) -> StoreResult<Self> {
        let boot_path = std::path::PathBuf::from("/proc/sys/kernel/random/boot_id");
        let boot = std::fs::read(&boot_path).map_err(|source| StoreError::Io {
            path: boot_path,
            source,
        })?;
        let task = Digest::parse(&format!("{:x}", Sha256::digest(owner.as_bytes())))?;
        let record = Self {
            owner: owner.into(),
            expires_at: expires_at.into(),
            epoch,
            revision: 1,
            owner_boot: Digest::parse(&format!("{:x}", Sha256::digest(&boot)))?,
            owner_task: task,
            object_root: Digest::parse(object_root)?,
            mode: LeaseMode::Write,
            reserved_bytes: 0,
        };
        record.validate()?;
        Ok(record)
    }
    fn validate(&self) -> StoreResult<()> {
        if self.owner.is_empty()
            || self.owner.len() > 128
            || self.owner.bytes().any(|b| b.is_ascii_control())
            || self.expires_at.len() > 64
            || self.epoch == 0
            || self.revision == 0
        {
            return Err(StoreError::Refused {
                detail: "lease requires a bounded owner, expiry and nonzero counters".into(),
            });
        }
        expiry(&self.expires_at)?;
        Ok(())
    }
}
impl Store {
    pub fn get_lease_record(&self, digest: &str) -> StoreResult<Option<LeaseRecord>> {
        check_digest(digest)?;
        self.lease_read(digest)
    }
    pub(super) fn lease_read(&self, digest: &str) -> StoreResult<Option<LeaseRecord>> {
        let key = lease_key(digest);
        let Some(bytes) = self
            .meta
            .get(key.as_bytes())
            .map_err(|source| StoreError::Read { source })?
        else {
            return Ok(None);
        };
        let lease: LeaseRecord = decode(&key, &bytes)?;
        lease.validate()?;
        if lease.object_root.as_str() != digest {
            return Err(StoreError::CorruptArtifact {
                digest: digest.into(),
            });
        }
        Ok(Some(lease))
    }
    pub(crate) fn lease_check(&self, digest: &str, epoch: u64) -> StoreResult<()> {
        let lease = self.lease_read(digest)?;
        let actual = lease.as_ref().map(|lease| lease.epoch);
        if actual != Some(epoch) || epoch == 0 {
            return Err(StoreError::LeaseMismatch {
                digest: digest.into(),
                expected: epoch,
                actual,
            });
        }
        let lease = lease.ok_or_else(|| StoreError::LeaseLost {
            digest: digest.into(),
            epoch,
        })?;
        if lease.mode == LeaseMode::Read {
            return Err(StoreError::LeaseLost {
                digest: digest.into(),
                epoch,
            });
        }
        if expiry(&lease.expires_at)? <= Utc::now() {
            return Err(StoreError::LeaseExpired {
                digest: digest.into(),
                epoch,
            });
        }
        Ok(())
    }
    fn lease_stage(
        &self,
        digest: &str,
        owner: &str,
        expires_at: &str,
        batch: &mut fjall::OwnedWriteBatch,
    ) -> StoreResult<u64> {
        check_digest(digest)?;
        if let Some(held) = self.lease_read(digest)? {
            return Err(StoreError::LeaseMismatch {
                digest: digest.into(),
                expected: 0,
                actual: Some(held.epoch),
            });
        }
        self.check_lease_capacity()?;
        self.lease_epoch_stage(digest, owner, expires_at, batch)
    }
    fn lease_epoch_stage(
        &self,
        digest: &str,
        owner: &str,
        expires_at: &str,
        batch: &mut fjall::OwnedWriteBatch,
    ) -> StoreResult<u64> {
        let current =
            meta::get_u64(&self.meta, "lease_epoch_counter")?.map_or(0, core::convert::identity);
        let epoch = current.checked_add(1).ok_or(StoreError::CounterExhausted)?;
        let lease = LeaseRecord::new(owner, expires_at, epoch, digest)?;
        if expiry(expires_at)? <= Utc::now() {
            return Err(StoreError::LeaseExpired {
                digest: digest.into(),
                epoch,
            });
        }
        meta::put_text(batch, &self.meta, "lease_epoch_counter", &epoch.to_string());
        batch.insert(&self.meta, lease_key(digest).as_bytes(), encode(&lease)?);
        Ok(epoch)
    }
    pub(super) fn lease_succeed_expired_stage(
        &self,
        digest: &str,
        owner: &str,
        expires_at: &str,
        expected_epoch: u64,
        batch: &mut fjall::OwnedWriteBatch,
    ) -> StoreResult<u64> {
        check_digest(digest)?;
        match self.lease_check(digest, expected_epoch) {
            Err(StoreError::LeaseExpired { .. }) => {}
            Err(error) => return Err(error),
            Ok(()) => {
                return Err(StoreError::LeaseMismatch {
                    digest: digest.into(),
                    expected: expected_epoch,
                    actual: Some(expected_epoch),
                })
            }
        }
        self.lease_epoch_stage(digest, owner, expires_at, batch)
    }
    pub fn lease_acquire(&self, digest: &str, owner: &str, expires_at: &str) -> StoreResult<u64> {
        let _appends = self.lock_appends();
        let mut batch = self.db.batch();
        let epoch = self.lease_stage(digest, owner, expires_at, &mut batch)?;
        batch
            .durability(Some(fjall::PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        Ok(epoch)
    }
    pub fn input_lease_acquire(
        &self,
        digest: &str,
        expected: u64,
        owner: &str,
        expires_at: &str,
    ) -> StoreResult<(u64, Transitioned)> {
        let _appends = self.lock_appends();
        let mut record = self.record_expected::<InputRecord>(digest, expected)?;
        let mut batch = self.db.batch();
        let epoch = self.lease_stage(digest, owner, expires_at, &mut batch)?;
        let revision = record
            .revision
            .checked_add(1)
            .ok_or(StoreError::CounterExhausted)?;
        record.lease_epoch = Some(epoch);
        record.revision = revision;
        batch.insert(
            &self.meta,
            key::<InputRecord>(digest).as_bytes(),
            encode(&record)?,
        );
        batch
            .durability(Some(fjall::PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        Ok((epoch, Transitioned { revision }))
    }
    fn check_lease_capacity(&self) -> StoreResult<()> {
        let held = self
            .meta
            .prefix(b"lease/")
            .take(MAX_DURABLE_LEASES)
            .try_fold(0_usize, |count, guard| {
                guard.key().map_err(|source| StoreError::Read { source })?;
                count.checked_add(1).ok_or(StoreError::CounterExhausted)
            })?;
        if held == MAX_DURABLE_LEASES {
            return Err(StoreError::BudgetExceeded {
                what: "durable leases",
                needed: 4097,
                available: 4096,
            });
        }
        Ok(())
    }
    pub fn lease_release(&self, digest: &str, epoch: u64) -> StoreResult<()> {
        check_digest(digest)?;
        let _appends = self.lock_appends();
        let actual = self.lease_read(digest)?.map(|lease| lease.epoch);
        if actual != Some(epoch) {
            return Err(StoreError::LeaseMismatch {
                digest: digest.into(),
                expected: epoch,
                actual,
            });
        }
        let mut batch = self.db.batch();
        batch.remove(&self.meta, lease_key(digest).as_bytes());
        batch
            .durability(Some(fjall::PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })
    }
}
