mod consume;
pub(in crate::public_identity) mod fence;
mod persist;
mod prepare;
mod preview;
mod witness;
pub(in crate::public_identity) use witness::MembershipWithholdingWitnessV1;

use crate::StoreRevision;
use census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdingRecordV1;
use census_domain::public_identity_history::{
    PreparedHistoryRecord, PublicAthleteId, RegistryHead,
};

pub struct PreparedMembershipWithholdingV1<'store> {
    record: LocalMembershipWithholdingRecordV1,
    expected: StoreRevision<'store>,
    history: PreparedHistoryRecord,
}
impl PreparedMembershipWithholdingV1<'_> {
    pub fn record(&self) -> &LocalMembershipWithholdingRecordV1 {
        &self.record
    }
}

pub struct MembershipWithholdingInventoryV1 {
    authority_context: String,
    affected_context: String,
    affected_record_keys: Vec<String>,
    claims: Vec<census_domain::public_identity_history::local_adjudication::RetainedClaimV1>,
}
impl MembershipWithholdingInventoryV1 {
    pub fn authority_context(&self) -> &str {
        &self.authority_context
    }
    pub fn affected_context(&self) -> &str {
        &self.affected_context
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
    head: RegistryHead,
    withheld_memberships: Vec<String>,
    replayed: bool,
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
