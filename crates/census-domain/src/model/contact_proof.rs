use super::canonical_json::serialized_digest;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use validation::{validate_claimed_digest, validate_structure};

pub const CONTACT_COLUMNS: &[&str; 11] = &[
    "school",
    "city",
    "state",
    "sport",
    "role",
    "coach_name",
    "public_professional_email",
    "ad_name",
    "ad_email",
    "source_url",
    "last_observed",
];
pub const CONTACT_PROOF_COLUMN: &str = "verified_proof_digest";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RawContactRow {
    pub school: String,
    pub city: String,
    pub state: String,
    pub sport: String,
    pub role: String,
    pub coach_name: String,
    pub public_professional_email: String,
    pub ad_name: String,
    pub ad_email: String,
    pub source_urls: Vec<String>,
    pub last_observed: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContactClaimEvidence {
    pub field: ContactProofField,
    pub value: String,
    pub person: String,
    pub role: String,
    pub sport: String,
    pub school: String,
    pub state: String,
    pub source_url: String,
    pub claimed_observed_on: String,
    pub source_sha256: String,
    pub fetched_at: String,
    pub span: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContactProofField {
    CoachName,
    PublicProfessionalEmail,
    AdName,
    AdEmail,
}

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

pub fn claim_binds_to_row(row: &RawContactRow, claim: &ContactClaimEvidence) -> bool {
    validation::claim_binds_to_row(row, claim)
}

pub fn is_director(field: ContactProofField) -> bool {
    validation::is_director(field)
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
