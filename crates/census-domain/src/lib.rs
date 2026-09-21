//! Pure domain model for the Midwest census.
//!
//! Dependency contract: this crate has no async runtime, no I/O, no HTTP client, no store engine
//! and no JSON value dependency. `tools/gate.sh` proves it with `cargo tree`; do not add
//! `tokio`, `fjall`, `reqwest`, `serde_json`, `restate-sdk` or `chromiumoxide` here for any
//! reason — if a function needs one of those, it does not belong in this crate. `thiserror` is a
//! compile-time proc-macro with no runtime dependency, so [`error`] may use it.

#![forbid(unsafe_code)]

pub mod error;
pub mod model;

#[cfg(kani)]
include!("../kani/census_domain_wiring.rs");

pub use error::DomainError;
