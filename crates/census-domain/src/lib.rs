#![forbid(unsafe_code)]

pub mod core_scope;
pub mod error;
pub mod jurisdiction;
pub mod model;
pub mod school_index;

pub use core_scope::{is_core_source, NON_CORE_SOURCE_IDS};
pub use error::DomainError;
pub use jurisdiction::{JurisdictionBucket, MeetState, UsJurisdiction};
pub use school_index::{SchoolIndex, SchoolMatch};

#[cfg(kani)]
include!("../kani/census_domain_wiring.rs");
