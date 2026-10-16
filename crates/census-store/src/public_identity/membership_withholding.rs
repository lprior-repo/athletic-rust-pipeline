pub(in crate::public_identity) mod consume;
pub(in crate::public_identity) mod fence;
mod persist;
mod prepare;
mod preview;
mod witness;
pub(in crate::public_identity) use witness::MembershipWithholdingWitnessV1;

use crate::StoreRevision;
use census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdingRecordV2;
use census_domain::public_identity_history::{
    PreparedHistoryRecord, PublicAthleteId, RegistryHead,
};

pub struct PreparedMembershipWithholdingV1<'store> {
    record: LocalMembershipWithholdingRecordV2,
    graph: super::graph_reference::GraphAuthorityReferenceV1,
    expected: StoreRevision<'store>,
    history: PreparedHistoryRecord,
}
impl PreparedMembershipWithholdingV1<'_> {
    pub fn record(&self) -> &LocalMembershipWithholdingRecordV2 {
        &self.record
    }
    pub fn graph(&self) -> &super::graph_reference::GraphAuthorityReferenceV1 {
        &self.graph
    }
    pub fn approval_bytes(&self) -> crate::StoreResult<Vec<u8>> {
        consume::graph_approval_bytes(&self.record, &self.graph)
    }
    pub fn approval_digest(&self) -> crate::StoreResult<String> {
        Ok(consume::graph_approval_digest(&self.approval_bytes()?))
    }
}

pub struct MembershipWithholdingInventoryV1 {
    graph: super::graph_reference::GraphAuthorityReferenceV1,
    affected_record_keys: Vec<String>,
    claims: Vec<census_domain::public_identity_history::local_adjudication::RetainedClaimV1>,
}
impl MembershipWithholdingInventoryV1 {
    pub fn graph(&self) -> &super::graph_reference::GraphAuthorityReferenceV1 {
        &self.graph
    }
    pub fn affected_record_keys(&self) -> &[String] {
        &self.affected_record_keys
    }
    pub fn claims(
        &self,
    ) -> &[census_domain::public_identity_history::local_adjudication::RetainedClaimV1] {
        &self.claims
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipWithholdingOutcomeV1 {
    pub(in crate::public_identity) head: RegistryHead,
    pub(in crate::public_identity) withheld_memberships: Vec<String>,
    pub(in crate::public_identity) replayed: bool,
}
impl MembershipWithholdingOutcomeV1 {
    pub fn head(&self) -> &RegistryHead {
        &self.head
    }
    pub fn withheld_memberships(&self) -> &[String] {
        &self.withheld_memberships
    }
    pub fn replayed(&self) -> bool {
        self.replayed
    }
    pub fn introduced_public_ids(&self) -> &[PublicAthleteId] {
        &[]
    }
}
