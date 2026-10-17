use super::{RosterAuthorityDigestV1, RosterAuthorityTextV1};
use crate::model::{
    ContractRefV1, Gender, ProfileDate, SchoolId, SchoolYear, SourceSubject, Sport, StoreIdentity,
    TeamId,
};
use serde::{Deserialize, Serialize};
use std::num::NonZeroU64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterAuthorityObjectRefV1 {
    pub(in crate::roster_affiliation_authority) digest: RosterAuthorityDigestV1,
    pub(in crate::roster_affiliation_authority) bytes: NonZeroU64,
}
impl RosterAuthorityObjectRefV1 {
    pub fn digest(&self) -> &RosterAuthorityDigestV1 {
        &self.digest
    }
    pub fn bytes(&self) -> NonZeroU64 {
        self.bytes
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::roster_affiliation_authority) enum Provider {
    Milesplit,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::roster_affiliation_authority) enum Relationship {
    RepresentsSchoolProgram,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::roster_affiliation_authority) enum Temporal {
    ExactUtcCaptureDay,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterAuthorityScopeV1 {
    pub(in crate::roster_affiliation_authority) provider: Provider,
    pub(in crate::roster_affiliation_authority) owner: SourceSubject,
    pub(in crate::roster_affiliation_authority) sport: Sport,
    pub(in crate::roster_affiliation_authority) category: Gender,
    pub(in crate::roster_affiliation_authority) policy: ContractRefV1,
    pub(in crate::roster_affiliation_authority) relationship: Relationship,
    pub(in crate::roster_affiliation_authority) temporal: Temporal,
}
impl RosterAuthorityScopeV1 {
    pub fn owner(&self) -> &SourceSubject {
        &self.owner
    }
    pub fn sport(&self) -> Sport {
        self.sport
    }
    pub fn category(&self) -> Gender {
        self.category
    }
    pub fn policy(&self) -> &ContractRefV1 {
        &self.policy
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterParserClaimV1 {
    pub(in crate::roster_affiliation_authority) artifact: RosterAuthorityObjectRefV1,
    pub(in crate::roster_affiliation_authority) interpretation: RosterAuthorityTextV1,
    pub(in crate::roster_affiliation_authority) schema: RosterAuthorityObjectRefV1,
    pub(in crate::roster_affiliation_authority) configuration: RosterAuthorityObjectRefV1,
    pub(in crate::roster_affiliation_authority) dependencies: RosterAuthorityObjectRefV1,
}
impl RosterParserClaimV1 {
    pub fn artifact(&self) -> &RosterAuthorityObjectRefV1 {
        &self.artifact
    }
    pub fn interpretation(&self) -> &str {
        self.interpretation.as_str()
    }
    pub fn schema(&self) -> &RosterAuthorityObjectRefV1 {
        &self.schema
    }
    pub fn configuration(&self) -> &RosterAuthorityObjectRefV1 {
        &self.configuration
    }
    pub fn dependencies(&self) -> &RosterAuthorityObjectRefV1 {
        &self.dependencies
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    content = "url",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub(in crate::roster_affiliation_authority) enum FinalUrl {
    Unknown,
    Observed(RosterAuthorityTextV1),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterCaptureClaimV1 {
    pub(in crate::roster_affiliation_authority) occurrence: RosterAuthorityDigestV1,
    pub(in crate::roster_affiliation_authority) body: RosterAuthorityObjectRefV1,
    pub(in crate::roster_affiliation_authority) manifest: RosterAuthorityObjectRefV1,
    pub(in crate::roster_affiliation_authority) interpretation: RosterAuthorityObjectRefV1,
    pub(in crate::roster_affiliation_authority) acquired_at: RosterAuthorityTextV1,
    pub(in crate::roster_affiliation_authority) observed_day: ProfileDate,
    pub(in crate::roster_affiliation_authority) requested_url: RosterAuthorityTextV1,
    pub(in crate::roster_affiliation_authority) final_url: FinalUrl,
    pub(in crate::roster_affiliation_authority) source_admission: RosterAuthorityObjectRefV1,
    pub(in crate::roster_affiliation_authority) original_grant: RosterAuthorityObjectRefV1,
}
impl RosterCaptureClaimV1 {
    pub fn occurrence(&self) -> &RosterAuthorityDigestV1 {
        &self.occurrence
    }
    pub fn body(&self) -> &RosterAuthorityObjectRefV1 {
        &self.body
    }
    pub fn manifest(&self) -> &RosterAuthorityObjectRefV1 {
        &self.manifest
    }
    pub fn interpretation(&self) -> &RosterAuthorityObjectRefV1 {
        &self.interpretation
    }
    pub fn acquired_at(&self) -> &str {
        self.acquired_at.as_str()
    }
    pub fn observed_day(&self) -> &ProfileDate {
        &self.observed_day
    }
    pub fn requested_url(&self) -> &str {
        self.requested_url.as_str()
    }
    pub fn observed_final_url(&self) -> Option<&str> {
        match &self.final_url {
            FinalUrl::Unknown => None,
            FinalUrl::Observed(url) => Some(url.as_str()),
        }
    }
    pub fn source_admission(&self) -> &RosterAuthorityObjectRefV1 {
        &self.source_admission
    }
    pub fn original_grant(&self) -> &RosterAuthorityObjectRefV1 {
        &self.original_grant
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RosterExistingBindingClaimV1 {
    pub(in crate::roster_affiliation_authority) school: SchoolId,
    pub(in crate::roster_affiliation_authority) program: TeamId,
    pub(in crate::roster_affiliation_authority) school_source: SourceSubject,
    pub(in crate::roster_affiliation_authority) program_source: SourceSubject,
    pub(in crate::roster_affiliation_authority) sport: Sport,
    pub(in crate::roster_affiliation_authority) category: Gender,
    pub(in crate::roster_affiliation_authority) school_year: SchoolYear,
    pub(in crate::roster_affiliation_authority) year_convention: ContractRefV1,
    pub(in crate::roster_affiliation_authority) school_row: RosterAuthorityObjectRefV1,
    pub(in crate::roster_affiliation_authority) program_row: RosterAuthorityObjectRefV1,
    pub(in crate::roster_affiliation_authority) provider_packet: RosterAuthorityObjectRefV1,
    pub(in crate::roster_affiliation_authority) conflict_inventory: RosterAuthorityObjectRefV1,
    pub(in crate::roster_affiliation_authority) store: StoreIdentity,
    pub(in crate::roster_affiliation_authority) snapshot_revision: NonZeroU64,
}
impl RosterExistingBindingClaimV1 {
    pub fn school(&self) -> &SchoolId {
        &self.school
    }
    pub fn program(&self) -> &TeamId {
        &self.program
    }
    pub fn school_source(&self) -> &SourceSubject {
        &self.school_source
    }
    pub fn program_source(&self) -> &SourceSubject {
        &self.program_source
    }
    pub fn sport(&self) -> Sport {
        self.sport
    }
    pub fn category(&self) -> Gender {
        self.category
    }
    pub fn school_year(&self) -> SchoolYear {
        self.school_year
    }
    pub fn year_convention(&self) -> &ContractRefV1 {
        &self.year_convention
    }
    pub fn school_row(&self) -> &RosterAuthorityObjectRefV1 {
        &self.school_row
    }
    pub fn program_row(&self) -> &RosterAuthorityObjectRefV1 {
        &self.program_row
    }
    pub fn provider_packet(&self) -> &RosterAuthorityObjectRefV1 {
        &self.provider_packet
    }
    pub fn conflict_inventory(&self) -> &RosterAuthorityObjectRefV1 {
        &self.conflict_inventory
    }
    pub fn store(&self) -> &StoreIdentity {
        &self.store
    }
    pub fn snapshot_revision(&self) -> NonZeroU64 {
        self.snapshot_revision
    }
}

mod records;
pub(in crate::roster_affiliation_authority) use records::{IssuanceWire, RecordWire};
pub use records::{LocallyCheckedRosterIssuanceV1, LocallyCheckedRosterRecordV1};
