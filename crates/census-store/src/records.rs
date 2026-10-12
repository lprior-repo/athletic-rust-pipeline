mod codec;
mod inventory;
mod lease;
mod model;
pub mod publish;
mod replay;
mod transition;
mod unit;

#[cfg(test)]
mod spec_tests;

use crate::{Store, StoreResult};
pub use model::{
    BuildRecord, BuildState, FailureKind, InputRecord, InputState, LeaseMode, LeaseRecord,
    PublishRecord, PublishState, Transitioned, UnitRecord, UnitState,
};

impl Store {
    pub fn reclaim_registered_objects(&self) -> StoreResult<()> {
        Err(crate::StoreError::ReferenceAccountingIncomplete)
    }
    pub fn reclaim_owner_dead_scratch(&self) -> StoreResult<usize> {
        let _appends = self.lock_appends();
        crate::read::sweep_stale_temporaries(self.root(), &self.meta)
    }
    pub fn put_input_record(&self, record: &InputRecord) -> StoreResult<()> {
        self.record_put(record)
    }
    pub fn get_input_record(&self, digest: &str) -> StoreResult<Option<InputRecord>> {
        self.record_read(digest)
    }
    pub fn put_build_record(&self, record: &BuildRecord) -> StoreResult<()> {
        self.record_put(record)
    }
    pub fn get_build_record(&self, digest: &str) -> StoreResult<Option<BuildRecord>> {
        self.record_read(digest)
    }
    pub fn put_unit_record(&self, record: &UnitRecord) -> StoreResult<()> {
        self.record_put(record)
    }
    pub fn get_unit_record(&self, digest: &str) -> StoreResult<Option<UnitRecord>> {
        self.record_read(digest)
    }
    pub fn put_publish_record(&self, record: &PublishRecord) -> StoreResult<()> {
        self.record_put(record)
    }
    pub fn get_publish_record(&self, digest: &str) -> StoreResult<Option<PublishRecord>> {
        self.record_read(digest)
    }
}
