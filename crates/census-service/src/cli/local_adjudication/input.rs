use census_domain::model::AthleteCandidateId;
use census_domain::public_identity_history::local_adjudication::{
    ClaimTreatmentV1, LocalIdentitySplitV1, RetainedClaimDispositionV1, RetainedClaimV1,
    RetainedIdentitySectionV1, SupersededMergeV1,
};
use census_domain::public_identity_history::{BindingPartition, HistoryError, PublicAthleteId};
use serde::Deserialize;
use std::path::Path;

const MAX_REQUEST_BYTES: usize = 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub(super) enum InputError {
    #[error("local identity request exceeds the {MAX_REQUEST_BYTES}-byte input ceiling")]
    Oversized,
    #[error("cannot read the bounded local identity request: {0}")]
    Read(#[from] census_crawl::CrawlError),
    #[error("local identity request JSON is malformed or contains unsupported fields: {0}")]
    Json(#[from] serde_json::Error),
    #[error("local identity request version {0} is unsupported; expected 1")]
    Version(u32),
    #[error("local identity request names an invalid public identity: {0}")]
    PublicId(#[from] HistoryError),
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SplitInput {
    version: u32,
    case_id: String,
    canonical: String,
    supersedes: Vec<SupersededInput>,
    partitions: Vec<PartitionInput>,
    unresolved_bindings: Vec<String>,
    affected_record_keys: Vec<String>,
    dispositions: Vec<DispositionInput>,
    rationale: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SupersededInput {
    sequence: u64,
    digest: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PartitionInput {
    binding_ids: Vec<String>,
    representative: AthleteCandidateId,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DispositionInput {
    claim: ClaimInput,
    treatment: TreatmentInput,
    rationale: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClaimInput {
    candidate: AthleteCandidateId,
    section: SectionInput,
    ordinal: u64,
    row_digest: String,
    claim_path: String,
    claim_digest: String,
    retained_value: serde_json::Value,
}

#[derive(Deserialize)]
enum SectionInput {
    Athlete,
    SourceObservation,
}

#[derive(Deserialize)]
enum TreatmentInput {
    RetainAsContext,
    RejectIdentityCorroboration,
}

pub(super) fn read_request(path: &Path) -> Result<LocalIdentitySplitV1, InputError> {
    parse_request(&census_crawl::convert::read_file_capped(
        path,
        MAX_REQUEST_BYTES,
    )?)
}

fn parse_request(bytes: &[u8]) -> Result<LocalIdentitySplitV1, InputError> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(InputError::Oversized);
    }
    let input: SplitInput = serde_json::from_slice(bytes)?;
    if input.version != 1 {
        return Err(InputError::Version(input.version));
    }
    Ok(LocalIdentitySplitV1 {
        case_id: input.case_id,
        canonical: PublicAthleteId::parse(&input.canonical)?,
        supersedes: input
            .supersedes
            .into_iter()
            .map(SupersededMergeV1::from)
            .collect(),
        partitions: input
            .partitions
            .into_iter()
            .map(BindingPartition::from)
            .collect(),
        unresolved_bindings: input.unresolved_bindings,
        affected_record_keys: input.affected_record_keys,
        dispositions: input
            .dispositions
            .into_iter()
            .map(RetainedClaimDispositionV1::from)
            .collect(),
        rationale: input.rationale,
    })
}

impl From<SupersededInput> for SupersededMergeV1 {
    fn from(value: SupersededInput) -> Self {
        Self {
            sequence: value.sequence,
            digest: value.digest,
        }
    }
}

impl From<PartitionInput> for BindingPartition {
    fn from(value: PartitionInput) -> Self {
        Self {
            binding_ids: value.binding_ids,
            representative: value.representative,
        }
    }
}

impl From<DispositionInput> for RetainedClaimDispositionV1 {
    fn from(value: DispositionInput) -> Self {
        Self {
            claim: value.claim.into(),
            treatment: match value.treatment {
                TreatmentInput::RetainAsContext => ClaimTreatmentV1::RetainAsContext,
                TreatmentInput::RejectIdentityCorroboration => {
                    ClaimTreatmentV1::RejectIdentityCorroboration
                }
            },
            rationale: value.rationale,
        }
    }
}

impl From<ClaimInput> for RetainedClaimV1 {
    fn from(value: ClaimInput) -> Self {
        Self {
            candidate: value.candidate,
            section: match value.section {
                SectionInput::Athlete => RetainedIdentitySectionV1::Athlete,
                SectionInput::SourceObservation => RetainedIdentitySectionV1::SourceObservation,
            },
            ordinal: value.ordinal,
            row_digest: value.row_digest,
            claim_path: value.claim_path,
            claim_digest: value.claim_digest,
            retained_value: value.retained_value,
        }
    }
}

#[cfg(test)]
mod tests;
