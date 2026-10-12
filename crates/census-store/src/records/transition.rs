use super::codec::{check_inventory, encode, key};
use super::model::*;
use crate::{Store, StoreError, StoreResult};

pub(super) fn state<T: Record, S: std::fmt::Debug + PartialEq>(
    record: &T,
    actual: S,
    expected: S,
) -> StoreResult<()> {
    if actual != expected {
        return Err(StoreError::RecordState {
            kind: T::KIND,
            digest: record.digest().into(),
            expected: format!("{expected:?}"),
            actual: format!("{actual:?}"),
        });
    }
    Ok(())
}
impl Store {
    pub(super) fn record_expected<T: Record>(&self, digest: &str, revision: u64) -> StoreResult<T> {
        let record = self
            .record_read::<T>(digest)?
            .ok_or_else(|| StoreError::RecordMissing {
                kind: T::KIND,
                digest: digest.into(),
            })?;
        if record.revision() != revision {
            return Err(StoreError::RecordRevision {
                kind: T::KIND,
                digest: digest.into(),
                expected: revision,
                actual: record.revision(),
            });
        }
        Ok(record)
    }
    pub(super) fn record_transition<T: Record>(
        &self,
        digest: &str,
        revision: u64,
        epoch: u64,
        transition: impl FnOnce(&mut T, &mut fjall::OwnedWriteBatch) -> StoreResult<()>,
    ) -> StoreResult<Transitioned> {
        let _appends = self.lock_appends();
        let mut record = self.record_expected::<T>(digest, revision)?;
        self.lease_check(digest, epoch)?;
        if record.lease() != Some(epoch) {
            return Err(StoreError::LeaseMismatch {
                digest: digest.into(),
                expected: epoch,
                actual: record.lease(),
            });
        }
        let mut batch = self.db.batch();
        transition(&mut record, &mut batch)?;
        let revision = record
            .revision()
            .checked_add(1)
            .ok_or(StoreError::CounterExhausted)?;
        *record.revision_mut() = revision;
        record.validate()?;
        batch.insert(&self.meta, key::<T>(digest).as_bytes(), encode(&record)?);
        batch
            .durability(Some(fjall::PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })?;
        Ok(Transitioned { revision })
    }
    pub fn input_freeze(
        &self,
        digest: &str,
        expected: u64,
        epoch: u64,
        chunks: &[String],
    ) -> StoreResult<Transitioned> {
        check_inventory(chunks)?;
        self.record_transition(digest, expected, epoch, |r: &mut InputRecord, _| {
            state(r, r.state, InputState::Requested)?;
            if r.chunks() != chunks {
                return Err(StoreError::RecordConflict {
                    kind: "input",
                    digest: digest.into(),
                });
            }
            r.state = InputState::Freezing;
            Ok(())
        })
    }
    pub fn input_frozen(
        &self,
        digest: &str,
        expected: u64,
        epoch: u64,
        bytes: u64,
        chunks: &[String],
    ) -> StoreResult<Transitioned> {
        check_inventory(chunks)?;
        self.record_transition(digest, expected, epoch, |r: &mut InputRecord, _| {
            state(r, r.state, InputState::Freezing)?;
            if r.bytes != bytes || r.chunks() != chunks {
                return Err(StoreError::RecordConflict {
                    kind: "input",
                    digest: digest.into(),
                });
            }
            r.state = InputState::Frozen;
            Ok(())
        })
    }
    pub fn input_abandon(
        &self,
        digest: &str,
        expected: u64,
        epoch: u64,
    ) -> StoreResult<Transitioned> {
        self.record_transition(digest, expected, epoch, |r: &mut InputRecord, _| {
            if !matches!(r.state, InputState::Requested | InputState::Freezing) {
                return Err(StoreError::RecordState {
                    kind: "input",
                    digest: digest.into(),
                    expected: "Requested or Freezing".into(),
                    actual: format!("{:?}", r.state),
                });
            }
            r.state = InputState::Abandoned;
            Ok(())
        })
    }
    pub fn build_start(
        &self,
        digest: &str,
        expected: u64,
        epoch: u64,
    ) -> StoreResult<Transitioned> {
        self.record_transition(digest, expected, epoch, |r: &mut BuildRecord, _| {
            state(r, r.state, BuildState::Frozen)?;
            let input = self
                .record_read::<InputRecord>(r.input_digest())?
                .ok_or_else(|| StoreError::RecordMissing {
                    kind: "input",
                    digest: r.input_digest().into(),
                })?;
            state(&input, input.state, InputState::Frozen)?;
            r.state = BuildState::Building;
            Ok(())
        })
    }
    pub fn build_complete(
        &self,
        digest: &str,
        expected: u64,
        epoch: u64,
    ) -> StoreResult<Transitioned> {
        self.record_transition(digest, expected, epoch, |r: &mut BuildRecord, _| {
            state(r, r.state, BuildState::Building)?;
            self.build_inventory_complete(r)?;
            r.state = BuildState::Built;
            Ok(())
        })
    }
    pub fn build_verify(
        &self,
        digest: &str,
        expected: u64,
        epoch: u64,
    ) -> StoreResult<Transitioned> {
        self.record_transition(digest, expected, epoch, |r: &mut BuildRecord, _| {
            state(r, r.state, BuildState::Built)?;
            self.build_inventory_complete(r)?;
            r.state = BuildState::Verifying;
            Ok(())
        })
    }
    pub fn build_verified(
        &self,
        digest: &str,
        expected: u64,
        epoch: u64,
    ) -> StoreResult<Transitioned> {
        self.record_transition(digest, expected, epoch, |r: &mut BuildRecord, _| {
            state(r, r.state, BuildState::Verifying)?;
            self.build_inventory_complete(r)?;
            r.state = BuildState::Verified;
            Ok(())
        })
    }
    pub fn build_rejected(
        &self,
        digest: &str,
        expected: u64,
        epoch: u64,
    ) -> StoreResult<Transitioned> {
        self.record_transition(digest, expected, epoch, |r: &mut BuildRecord, _| {
            state(r, r.state, BuildState::Verifying)?;
            r.state = BuildState::Rejected;
            Ok(())
        })
    }
}
