use crate::public_identity::{context, PublicIdentityError};
use crate::StoreResult;
use census_domain::model::StoreIdentity;
use serde::{Deserialize, Serialize};

mod live;
pub(super) use live::Live;

#[derive(Clone, Copy)]
pub(super) enum Rows {
    Witness,
    Receipt,
    Decision,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Revision {
    pub evidence_generation: u64,
    pub derived_generation: u64,
    pub snapshot_sequence: u64,
}

impl Revision {
    pub(super) fn includes(&self, prior: Self) -> StoreResult<()> {
        if self.evidence_generation <= prior.evidence_generation
            || self.derived_generation < prior.derived_generation
            || self.snapshot_sequence <= prior.snapshot_sequence
        {
            return Err(
                PublicIdentityError::Corrupt("commit revision exceeds retained state").into(),
            );
        }
        Ok(())
    }
}

pub(super) trait Read {
    fn row(&self, rows: Rows, key: &str) -> StoreResult<Option<Vec<u8>>>;
    fn context(&self, root: &str, identity: &StoreIdentity) -> StoreResult<context::Inputs>;
    fn affected_context(
        &self,
        root: &str,
    ) -> StoreResult<super::local_adjudication::affected::Inputs>;
    fn revision(&self) -> StoreResult<Revision>;
}
