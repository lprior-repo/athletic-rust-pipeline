mod atoms;
mod member_evidence;
mod member_observation;
mod types;
mod validate;
pub use atoms::{RosterAuthorityDigestV1, RosterAuthorityTextV1};
pub use member_evidence::{
    check_roster_member_witness_v1, encode_roster_member_witness_v1, CheckedRosterMemberEvidenceV1,
    SourceRowLocator,
};
pub use member_observation::RosterMemberObservationV1;
use serde::Serialize;
pub use types::{
    LocallyCheckedRosterIssuanceV1, LocallyCheckedRosterProposalV1, LocallyCheckedRosterRecordV1,
    RosterAuthorityObjectRefV1, RosterAuthorityScopeV1, RosterCaptureClaimV1,
    RosterExistingBindingClaimV1, RosterParserClaimV1,
};

pub const MAX_ROSTER_AUTHORITY_RECORD_BYTES: usize = 64 * 1024;
pub const MAX_ROSTER_AUTHORITY_COMPONENT_BYTES: u64 = 8 * 1024 * 1024;
const MAX_BODY_BYTES: u64 = 4 * 1024 * 1024;
const MAX_OBJECT_BYTES: u64 = 64 * 1024;

type Result<T> = std::result::Result<T, RosterAuthorityError>;

#[derive(Debug, thiserror::Error)]
pub enum RosterAuthorityError {
    #[error("invalid roster authority {0}")]
    Invalid(&'static str),
    #[error("unsupported roster authority version")]
    UnsupportedVersion,
    #[error("unsupported roster authority lifecycle action")]
    UnsupportedLifecycle,
    #[error("roster acquisition UTC day differs from the asserted observation day")]
    AcquisitionDayMismatch,
    #[error("roster scope and existing program binding differ")]
    BindingScopeMismatch,
    #[error("roster object digest has conflicting byte lengths")]
    ConflictingObjectReference,
    #[error("roster authority {resource} capacity: needed {needed}, available {available}")]
    Capacity {
        resource: &'static str,
        needed: u64,
        available: u64,
    },
    #[error("roster authority arithmetic overflow")]
    Overflow,
    #[error("roster authority allocation refused")]
    Allocation,
    #[error("roster authority decoding: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("roster authority canonical encoding: {0}")]
    Encode(#[from] crate::model::CanonicalJsonError),
}

#[derive(Debug, Clone, Copy)]
pub struct RosterAuthorityLimitsV1 {
    record_bytes: usize,
    component_bytes: u64,
}

impl RosterAuthorityLimitsV1 {
    pub fn new(record_bytes: usize, component_bytes: u64) -> Result<Self> {
        if record_bytes == 0 || component_bytes == 0 {
            return Err(RosterAuthorityError::Invalid("zero read grant"));
        }
        charge(
            "record bytes",
            size(record_bytes)?,
            size(MAX_ROSTER_AUTHORITY_RECORD_BYTES)?,
        )?;
        charge(
            "component bytes",
            component_bytes,
            MAX_ROSTER_AUTHORITY_COMPONENT_BYTES,
        )?;
        Ok(Self {
            record_bytes,
            component_bytes,
        })
    }
}

pub fn decode_roster_issuance_claim(
    bytes: &[u8],
    limits: RosterAuthorityLimitsV1,
) -> Result<LocallyCheckedRosterIssuanceV1> {
    admit(bytes, limits)?;
    let raw: types::IssuanceWire = serde_json::from_slice(bytes)?;
    validate::issuance(&raw, limits)?;
    Ok(LocallyCheckedRosterIssuanceV1(raw))
}
pub fn decode_roster_issuance_proposal_v1(
    bytes: &[u8],
    limits: RosterAuthorityLimitsV1,
) -> Result<LocallyCheckedRosterProposalV1> {
    admit(bytes, limits)?;
    let raw: types::ProposalWire = serde_json::from_slice(bytes)?;
    validate::proposal(&raw, limits)?;
    Ok(LocallyCheckedRosterProposalV1(raw))
}

pub fn decode_roster_record_claim(
    bytes: &[u8],
    limits: RosterAuthorityLimitsV1,
) -> Result<LocallyCheckedRosterRecordV1> {
    admit(bytes, limits)?;
    let raw: types::RecordWire = serde_json::from_slice(bytes)?;
    validate::record(&raw, limits)?;
    Ok(LocallyCheckedRosterRecordV1(raw))
}

fn admit(bytes: &[u8], limits: RosterAuthorityLimitsV1) -> Result<()> {
    if bytes.is_empty() {
        return Err(RosterAuthorityError::Invalid("empty record"));
    }
    charge(
        "record bytes",
        size(bytes.len())?,
        size(limits.record_bytes)?,
    )
}

fn encode(value: &impl Serialize, limits: RosterAuthorityLimitsV1) -> Result<Vec<u8>> {
    let bytes = crate::model::serialized_bytes(value)?;
    admit(&bytes, limits)?;
    Ok(bytes)
}

fn size(value: usize) -> Result<u64> {
    u64::try_from(value).map_err(|_| RosterAuthorityError::Overflow)
}

fn charge(resource: &'static str, needed: u64, available: u64) -> Result<()> {
    if needed > available {
        return Err(RosterAuthorityError::Capacity {
            resource,
            needed,
            available,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
