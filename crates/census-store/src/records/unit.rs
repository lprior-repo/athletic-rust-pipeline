use super::model::*;
use super::transition::state;
use crate::clock::{Clock, SystemClock};
use crate::{Store, StoreError, StoreResult};
use sha2::{Digest as _, Sha256};

impl Store {
    pub fn unit_start(&self, digest: &str, expected: u64, epoch: u64) -> StoreResult<Transitioned> {
        self.record_transition(digest, expected, epoch, |r: &mut UnitRecord, _| {
            state(r, r.state.clone(), UnitState::Pending)?;
            self.unit_build_running(r)?;
            r.state = UnitState::Running;
            Ok(())
        })
    }
    pub fn unit_complete(
        &self,
        digest: &str,
        expected: u64,
        epoch: u64,
        output: &[u8],
    ) -> StoreResult<Transitioned> {
        if output.len() > super::codec::MAX_RECORD_BYTES {
            return Err(StoreError::RowTooLarge {
                bytes: output.len(),
                max: super::codec::MAX_RECORD_BYTES,
            });
        }
        let output_digest =
            super::publish::Digest::parse(&format!("{:x}", Sha256::digest(output)))?;
        self.record_transition(digest, expected, epoch, |r: &mut UnitRecord, batch| {
            state(r, r.state.clone(), UnitState::Running)?;
            self.unit_build_running(r)?;
            batch.insert(
                &self.meta,
                super::replay::output_key(&output_digest).as_bytes(),
                output,
            );
            let receipt = crate::Receipt {
                operation: super::replay::receipt_key(digest),
                digest: output_digest.as_str().into(),
                at: SystemClock.today_iso8601(),
                appended: 0,
                credited: false,
            };
            batch.insert(
                &self.receipts,
                receipt.operation.as_bytes(),
                super::codec::encode(&receipt)?,
            );
            r.state = UnitState::Complete {
                output: output_digest,
            };
            Ok(())
        })
    }
    pub fn unit_fail(
        &self,
        digest: &str,
        expected: u64,
        epoch: u64,
        failure: FailureKind,
    ) -> StoreResult<Transitioned> {
        self.record_transition(digest, expected, epoch, |r: &mut UnitRecord, _| {
            state(r, r.state.clone(), UnitState::Running)?;
            r.state = UnitState::Failed { failure };
            Ok(())
        })
    }
    pub fn publish_complete(
        &self,
        digest: &str,
        expected: u64,
        epoch: u64,
    ) -> StoreResult<Transitioned> {
        self.record_transition(digest, expected, epoch, |r: &mut PublishRecord, _| {
            state(r, r.state, PublishState::Intent)?;
            self.publication_unit_verified(r.unit_digest())?;
            r.state = PublishState::Published;
            Ok(())
        })
    }
    pub fn publish_supersede(
        &self,
        digest: &str,
        expected: u64,
        epoch: u64,
    ) -> StoreResult<Transitioned> {
        self.record_transition(digest, expected, epoch, |r: &mut PublishRecord, _| {
            state(r, r.state, PublishState::Intent)?;
            r.state = PublishState::Superseded;
            Ok(())
        })
    }
    pub fn publish_recovery_required(
        &self,
        digest: &str,
        expected: u64,
        epoch: u64,
    ) -> StoreResult<Transitioned> {
        self.record_transition(digest, expected, epoch, |r: &mut PublishRecord, _| {
            state(r, r.state, PublishState::Intent)?;
            r.state = PublishState::RecoveryRequired;
            Ok(())
        })
    }
}
