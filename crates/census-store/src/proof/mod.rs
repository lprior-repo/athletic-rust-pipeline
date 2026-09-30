use crate::digest::serialized_digest;
use census_domain::model::{ContactClaimEvidence, RawContactRow};
use serde::Serialize;
use thiserror::Error;
use validation::{validate_claimed_digest, validate_structure};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedContactProof {
    proof: String,
}

impl ValidatedContactProof {
    pub fn as_str(&self) -> &str {
        &self.proof
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ContactProofError {
    #[error("claimed proof digest is not 64-char lowercase hex")]
    MalformedClaimedDigest,
    #[error("source_sha256 is not 64-char hex")]
    MalformedSourceHash,
    #[error("empty source URL")]
    EmptySourceUrl,
    #[error("span exceeds {0} bytes")]
    SpanTooLong(usize),
    #[error("source_url exceeds {0} bytes")]
    SourceUrlTooLong(usize),
    #[error("too many claims: {0} > {1}")]
    TooManyClaims(usize, usize),
    #[error("unsupported claim field")]
    UnsupportedField,
    #[error("claim value mismatch for field: {0}")]
    ValueMismatch(String),
    #[error("person mismatch for coach_name")]
    CoachPersonMismatch,
    #[error("person mismatch for ad_name")]
    AdNamePersonMismatch,
    #[error("missing person for email field")]
    MissingEmailPerson,
    #[error("claim cannot verify: empty or missing")]
    UnverifiedClaim,
    #[error("fetched_at not RFC3339")]
    FetchedAtInvalid,
    #[error("claimed_observed_on not YYYY-MM-DD")]
    ClaimedObservedOnInvalid,
    #[error("last_observed not YYYY-MM-DD")]
    LastObservedInvalid,
    #[error("claimed date exceeds fetched date")]
    ClaimedAfterFetched,
    #[error("source URL not in row")]
    SourceUrlNotFound,
    #[error("populated field {0} has no covering claim")]
    ClaimFieldNotCovered(String),
    #[error("proof digest mismatch")]
    DigestMismatch,
    #[error("serialization error")]
    SerializationError,
}

pub(super) const MAX_CLAIMS: usize = 256;
pub(super) const MAX_SPAN: usize = 4096;
pub(super) const MAX_URL: usize = 4096;

#[derive(Debug, Serialize)]
struct ContactProofPayload<'a> {
    row: &'a RawContactRow,
    claims: &'a [ContactClaimEvidence],
    policy: u32,
}

pub fn compute_contact_proof(
    row: &RawContactRow,
    claims: &[ContactClaimEvidence],
) -> Result<String, ContactProofError> {
    validate_structure(row, claims)?;
    compute_digest(row, claims)
}

pub fn verify_contact_proof(
    row: &RawContactRow,
    claims: &[ContactClaimEvidence],
    claimed_digest: &str,
) -> Result<ValidatedContactProof, ContactProofError> {
    validate_claimed_digest(claimed_digest)?;
    validate_structure(row, claims)?;
    let actual = compute_digest(row, claims)?;
    if actual != claimed_digest {
        return Err(ContactProofError::DigestMismatch);
    }
    Ok(ValidatedContactProof { proof: actual })
}

fn compute_digest(
    row: &RawContactRow,
    claims: &[ContactClaimEvidence],
) -> Result<String, ContactProofError> {
    let payload = ContactProofPayload {
        row,
        claims,
        policy: 1,
    };
    serialized_digest(&payload).map_err(|_| ContactProofError::SerializationError)
}
mod validation;

#[cfg(test)]
mod tests;
