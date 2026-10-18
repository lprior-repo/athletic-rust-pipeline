mod archive;
mod capture;
pub(super) mod enrollment;
mod source;
use super::codec::bounded as encoding;
mod replay;
mod retained;

pub use enrollment::{replay_result_school_enrollment_proof_v1, ResultSchoolEnrollmentProofV1};
pub use replay::replay_result_school_source_proof_v1;
pub use source::{ReplayedResultSchoolCorrespondencesV1, ResultSchoolSourceProofV1};
