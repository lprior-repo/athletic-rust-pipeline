use super::canonical_json::serialized_digest;
use serde::{Deserialize, Serialize};

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
