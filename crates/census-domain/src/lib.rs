//! Pure domain model for the national census.
//!
//! Dependency contract: this crate has no async runtime, no I/O, no HTTP client, no store engine
//! and no JSON value dependency. `tools/gate.sh` proves it with `cargo tree`; do not add
//! `tokio`, `fjall`, `reqwest`, `serde_json`, `restate-sdk` or `chromiumoxide` here for any
//! reason — if a function needs one of those, it does not belong in this crate. `thiserror` is a
//! compile-time proc-macro with no runtime dependency, so [`error`] may use it.

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
