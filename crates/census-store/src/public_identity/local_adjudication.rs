pub(in crate::public_identity) mod admission;
pub(in crate::public_identity) mod affected;
pub(in crate::public_identity) mod claims;
mod consume;
pub(in crate::public_identity) mod fence;
pub(in crate::public_identity) mod prepare;
pub(in crate::public_identity) use consume::{consume, ApprovedLocalAdjudicationV1};

use crate::StoreRevision;
use census_domain::public_identity_history::local_adjudication::{
    LocalIdentityAdjudicationRecordV2, RetainedClaimV1,
};
use census_domain::public_identity_history::PreparedHistoryRecord;

pub struct PreparedLocalAdjudicationV1<'store> {
    record: LocalIdentityAdjudicationRecordV2,
    expected: StoreRevision<'store>,
    history: PreparedHistoryRecord,
}

impl PreparedLocalAdjudicationV1<'_> {
    pub fn record(&self) -> &LocalIdentityAdjudicationRecordV2 {
        &self.record
    }
}

pub struct LocalAdjudicationInventoryV1 {
    authority_context: String,
    affected_context: String,
    affected_record_keys: Vec<String>,
    claims: Vec<RetainedClaimV1>,
}

impl LocalAdjudicationInventoryV1 {
    pub fn authority_context(&self) -> &str {
        &self.authority_context
    }
    pub fn affected_context(&self) -> &str {
        &self.affected_context
    }
    pub fn affected_record_keys(&self) -> &[String] {
        &self.affected_record_keys
    }
    pub fn claims(&self) -> &[RetainedClaimV1] {
        &self.claims
    }
}

mod persist;
mod witness;
pub(in crate::public_identity) use witness::LocalAdjudicationWitnessV1;
pub(in crate::public_identity) const MAX_WITNESS_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalAdjudicationCommitOutcomeV1 {
    introduced_public_ids: Vec<census_domain::public_identity_history::PublicAthleteId>,
    head: census_domain::public_identity_history::RegistryHead,
    replayed: bool,
}
impl LocalAdjudicationCommitOutcomeV1 {
    pub fn introduced_public_ids(
        &self,
    ) -> &[census_domain::public_identity_history::PublicAthleteId] {
        &self.introduced_public_ids
    }
    pub fn head(&self) -> &census_domain::public_identity_history::RegistryHead {
        &self.head
    }
    pub fn replayed(&self) -> bool {
        self.replayed
    }
}
