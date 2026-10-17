use super::*;
use crate::roster_affiliation_authority::{Result, RosterAuthorityLimitsV1};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::roster_affiliation_authority) struct IssuanceWire {
    pub version: u32,
    pub scope: RosterAuthorityScopeV1,
    pub parser: RosterParserClaimV1,
    pub capture: RosterCaptureClaimV1,
    pub binding: RosterExistingBindingClaimV1,
    pub semantic_review_evidence: RosterAuthorityObjectRefV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct LocallyCheckedRosterIssuanceV1(pub(in crate::roster_affiliation_authority) IssuanceWire);
impl LocallyCheckedRosterIssuanceV1 {
    pub fn scope(&self) -> &RosterAuthorityScopeV1 { &self.0.scope }
    pub fn parser(&self) -> &RosterParserClaimV1 { &self.0.parser }
    pub fn capture(&self) -> &RosterCaptureClaimV1 { &self.0.capture }
    pub fn binding(&self) -> &RosterExistingBindingClaimV1 { &self.0.binding }
    pub fn semantic_review_evidence(&self) -> &RosterAuthorityObjectRefV1 { &self.0.semantic_review_evidence }
    pub fn encode(&self, limits: RosterAuthorityLimitsV1) -> Result<Vec<u8>> {
        crate::roster_affiliation_authority::validate::issuance(&self.0, limits)?;
        crate::roster_affiliation_authority::encode(self, limits)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::roster_affiliation_authority) struct RecordWire {
    pub version: u32,
    pub lifecycle: RosterAuthorityTextV1,
    pub operation: RosterAuthorityTextV1,
    pub proposal: IssuanceWire,
    pub store: StoreIdentity,
    pub snapshot_revision: NonZeroU64,
    pub parent_inventory: RosterAuthorityObjectRefV1,
    pub negative_inventory: RosterAuthorityObjectRefV1,
    pub negative_actions: u32,
    pub approval_envelope: RosterAuthorityObjectRefV1,
    pub review_provenance: RosterAuthorityObjectRefV1,
    pub issuance_receipt: RosterAuthorityObjectRefV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct LocallyCheckedRosterRecordV1(pub(in crate::roster_affiliation_authority) RecordWire);
impl LocallyCheckedRosterRecordV1 {
    pub fn scope(&self) -> &RosterAuthorityScopeV1 { &self.0.proposal.scope }
    pub fn parser(&self) -> &RosterParserClaimV1 { &self.0.proposal.parser }
    pub fn capture(&self) -> &RosterCaptureClaimV1 { &self.0.proposal.capture }
    pub fn binding(&self) -> &RosterExistingBindingClaimV1 { &self.0.proposal.binding }
    pub fn semantic_review_evidence(&self) -> &RosterAuthorityObjectRefV1 { &self.0.proposal.semantic_review_evidence }
    pub fn operation(&self) -> &str { self.0.operation.as_str() }
    pub fn store(&self) -> &StoreIdentity { &self.0.store }
    pub fn snapshot_revision(&self) -> NonZeroU64 { self.0.snapshot_revision }
    pub fn parent_inventory(&self) -> &RosterAuthorityObjectRefV1 { &self.0.parent_inventory }
    pub fn negative_inventory(&self) -> &RosterAuthorityObjectRefV1 { &self.0.negative_inventory }
    pub fn approval_envelope(&self) -> &RosterAuthorityObjectRefV1 { &self.0.approval_envelope }
    pub fn review_provenance(&self) -> &RosterAuthorityObjectRefV1 { &self.0.review_provenance }
    pub fn issuance_receipt(&self) -> &RosterAuthorityObjectRefV1 { &self.0.issuance_receipt }
    pub fn encode(&self, limits: RosterAuthorityLimitsV1) -> Result<Vec<u8>> {
        crate::roster_affiliation_authority::validate::record(&self.0, limits)?;
        crate::roster_affiliation_authority::encode(self, limits)
    }
}
