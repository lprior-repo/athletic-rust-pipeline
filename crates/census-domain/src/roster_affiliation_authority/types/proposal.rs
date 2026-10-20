use super::*;
use crate::roster_affiliation_authority::{
    validate, Result, RosterAuthorityError, RosterAuthorityLimitsV1,
};
use std::num::NonZeroU64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::roster_affiliation_authority) struct ProposalWire {
    pub version: u32,
    pub scope: RosterAuthorityScopeV1,
    pub parser: RosterParserClaimV1,
    pub capture: RosterCaptureClaimV1,
    pub binding: RosterExistingBindingClaimV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct LocallyCheckedRosterProposalV1(pub(in crate::roster_affiliation_authority) ProposalWire);

impl LocallyCheckedRosterProposalV1 {
    pub fn scope(&self) -> &RosterAuthorityScopeV1 {
        &self.0.scope
    }

    pub fn parser(&self) -> &RosterParserClaimV1 {
        &self.0.parser
    }

    pub fn capture(&self) -> &RosterCaptureClaimV1 {
        &self.0.capture
    }

    pub fn binding(&self) -> &RosterExistingBindingClaimV1 {
        &self.0.binding
    }

    pub fn encode(&self, limits: RosterAuthorityLimitsV1) -> Result<Vec<u8>> {
        validate::proposal(&self.0, limits)?;
        crate::roster_affiliation_authority::encode(self, limits)
    }

    pub fn into_issuance(
        self,
        digest: &str,
        bytes: u64,
        limits: RosterAuthorityLimitsV1,
    ) -> Result<LocallyCheckedRosterIssuanceV1> {
        let review = RosterAuthorityObjectRefV1 {
            digest: RosterAuthorityDigestV1::parse(digest)?,
            bytes: NonZeroU64::new(bytes)
                .ok_or(RosterAuthorityError::Invalid("empty review object"))?,
        };
        let issuance = LocallyCheckedRosterIssuanceV1(IssuanceWire {
            version: self.0.version,
            scope: self.0.scope,
            parser: self.0.parser,
            capture: self.0.capture,
            binding: self.0.binding,
            semantic_review_evidence: review,
        });
        validate::issuance(&issuance.0, limits)?;
        crate::roster_affiliation_authority::encode(&issuance, limits)?;
        Ok(issuance)
    }
}

impl LocallyCheckedRosterIssuanceV1 {
    pub fn into_proposal(self) -> LocallyCheckedRosterProposalV1 {
        LocallyCheckedRosterProposalV1(ProposalWire {
            version: self.0.version,
            scope: self.0.scope,
            parser: self.0.parser,
            capture: self.0.capture,
            binding: self.0.binding,
        })
    }
}
