#![forbid(unsafe_code)]

#[cfg(test)]
#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

pub mod affiliation_candidates;
pub mod core_scope;
pub mod error;
pub mod evidence_subject_attachment;
pub mod jurisdiction;
pub mod model;
pub mod national_census;
pub mod participation_candidates;
pub mod profile_execution;
pub mod public_identity_history;
pub mod public_identity_qualification;
pub mod result_school_correspondence;
pub mod review_decision;
pub mod review_model;
pub mod roster_affiliation_authority;
pub mod school_directory;
pub mod school_index;

pub use core_scope::{is_core_source, NON_CORE_SOURCE_IDS};
pub use error::DomainError;
pub use jurisdiction::{JurisdictionBucket, MeetState, UsJurisdiction};
pub use school_index::{SchoolIndex, SchoolMatch};
