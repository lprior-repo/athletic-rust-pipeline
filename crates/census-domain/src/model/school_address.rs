use super::{CanonicalSchool, Evidence, SourceIdentity};
use crate::school_directory::{PostalAddress, SourceLabel};
use serde::{Deserialize, Serialize};

mod boundary;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "SchoolPostalAddressWire")]
pub struct SchoolPostalAddress {
    address: PostalAddress,
    owner: SourceIdentity,
    source_label: SourceLabel,
    evidence: Evidence,
    capture_sha256: String,
}

#[derive(Deserialize)]
struct SchoolPostalAddressWire {
    address: PostalAddress,
    owner: SourceIdentity,
    source_label: SourceLabel,
    evidence: Evidence,
    capture_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SchoolAddressError {
    #[error("postal claim has no published street")]
    MissingStreet,
    #[error("postal claim has no bounded school-provider identity")]
    MissingOwner,
    #[error("postal claim has no parsed capture URL and observation date")]
    MissingParsedCapture,
    #[error("postal claim capture SHA256 must contain 64 lowercase hexadecimal digits")]
    InvalidCaptureDigest,
    #[error("postal claim provider identity is not attached to this school")]
    ForeignOwner,
    #[error("postal claim authority is not an admitted school association source")]
    UnsupportedAuthority,
    #[error("postal claim source does not match its school-provider namespace")]
    SourceAuthorityMismatch,
    #[error("postal claim observation date is not a bounded ISO date or RFC3339 instant")]
    InvalidObservationDate,
    #[error("postal claim source URL is not a bounded HTTP(S) URL")]
    InvalidSourceUrl,
    #[error("postal claim provenance contains invalid or over-budget text")]
    InvalidProvenance,
    #[error("postal claim jurisdiction contradicts its source or school")]
    ForeignJurisdiction,
}

impl SchoolPostalAddress {
    pub fn new(
        address: PostalAddress,
        owner: SourceIdentity,
        source_label: SourceLabel,
        evidence: Evidence,
        capture_sha256: String,
    ) -> Result<Self, SchoolAddressError> {
        if address.line1().is_none() {
            return Err(SchoolAddressError::MissingStreet);
        }
        let state = boundary::provenance(&owner, &source_label, &evidence)?;
        if capture_sha256.len() != 64
            || !capture_sha256
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(SchoolAddressError::InvalidCaptureDigest);
        }
        if address.state().is_some_and(|actual| actual != state) {
            return Err(SchoolAddressError::ForeignJurisdiction);
        }
        Ok(Self {
            address,
            owner,
            source_label,
            evidence,
            capture_sha256,
        })
    }

    pub fn address(&self) -> &PostalAddress {
        &self.address
    }
    pub fn owner(&self) -> &SourceIdentity {
        &self.owner
    }
    pub fn source_label(&self) -> &SourceLabel {
        &self.source_label
    }
    pub fn evidence(&self) -> &Evidence {
        &self.evidence
    }
    pub fn capture_sha256(&self) -> &str {
        &self.capture_sha256
    }

    pub fn belongs_to(&self, school: &CanonicalSchool) -> Result<(), SchoolAddressError> {
        if !school
            .source_identities
            .iter()
            .any(|owner| owner.namespace == self.owner.namespace && owner.id == self.owner.id)
        {
            return Err(SchoolAddressError::ForeignOwner);
        }
        let source_state = boundary::source_state(&self.source_label)?;
        if school.state.is_some_and(|state| state != source_state) {
            return Err(SchoolAddressError::ForeignJurisdiction);
        }
        if school
            .state
            .zip(self.address.state())
            .is_some_and(|(school, address)| school != address)
        {
            return Err(SchoolAddressError::ForeignJurisdiction);
        }
        Ok(())
    }
}

impl TryFrom<SchoolPostalAddressWire> for SchoolPostalAddress {
    type Error = SchoolAddressError;

    fn try_from(wire: SchoolPostalAddressWire) -> Result<Self, Self::Error> {
        Self::new(
            wire.address,
            wire.owner,
            wire.source_label,
            wire.evidence,
            wire.capture_sha256,
        )
    }
}

impl Ord for SchoolPostalAddress {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        (
            &self.address,
            &self.owner,
            &self.source_label,
            &self.evidence.source,
            &self.evidence.observed_on,
            &self.evidence.note,
            &self.capture_sha256,
        )
            .cmp(&(
                &other.address,
                &other.owner,
                &other.source_label,
                &other.evidence.source,
                &other.evidence.observed_on,
                &other.evidence.note,
                &other.capture_sha256,
            ))
    }
}

impl PartialOrd for SchoolPostalAddress {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub(super) fn deserialize_postal_addresses<'de, D>(
    deserializer: D,
) -> Result<Vec<SchoolPostalAddress>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let mut claims = Vec::<SchoolPostalAddress>::deserialize(deserializer)?;
    claims.sort_unstable();
    claims.dedup();
    Ok(claims)
}

#[cfg(test)]
#[path = "school_address_tests.rs"]
mod tests;
