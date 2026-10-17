use census_store::{StoreError, StoreResult};
use sha2::{Digest, Sha256};

mod bundle;
mod capture;
mod codec;
mod markup;
mod packet;
pub mod physical_result;
pub mod provider_school;
pub mod provider_target;
pub mod recorded;
mod result_context;
pub mod result_ownership;
pub mod roster_source_issuance;
mod schedule;
mod status;
mod transport;
mod types;

pub use bundle::IndependentParticipationBundleV1;
pub use capture::RetainedHtmlCaptureV1;
pub use packet::IndependentParticipationPacketV1;
pub use result_context::retained_result_venue;
pub use transport::{RetainedBodyEncodingV1, RetainedHttpProvenanceV1};
pub use types::{
    DocumentSubjectV1, IndependentParticipationProofV1, ParticipationEvidenceRefV1,
    SubjectOwnershipV1, VerifiedIndependentParticipationV1,
};

pub const PARTICIPATION_EVIDENCE_PHASE: &str = "independent_participation_capture_v1";
const MAX_BODY: usize = 512 * 1024;
const MAX_PACKET: usize = 2 * 1024 * 1024;
const MAX_PACKETS: usize = 16;
const MAX_BUNDLE: usize = 32 * 1024 * 1024;
const INTERPRETATION: &str = "mhsaa_subject_dcc_event_day_v1";

fn invalid(detail: &'static str) -> StoreError {
    StoreError::Refused {
        detail: detail.into(),
    }
}

fn bound(needed: usize, limit: usize, what: &'static str) -> StoreResult<()> {
    if needed > limit {
        return Err(StoreError::BudgetExceeded {
            what,
            needed: u64::try_from(needed).map_err(|_| StoreError::CounterOverflow)?,
            available: u64::try_from(limit).map_err(|_| StoreError::CounterOverflow)?,
        });
    }
    Ok(())
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn json(source: serde_json::Error) -> StoreError {
    StoreError::Json {
        detail: "independent participation evidence".into(),
        source,
    }
}

#[cfg(test)]
mod tests;
