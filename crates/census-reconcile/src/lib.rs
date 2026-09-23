//! `census-reconcile` — the reconciliation lane (§17): deterministic identity, and the row-level
//! check that holds the published workbook against the store.
//!
//! * [`identity`] — every workflow's address derived from explicit values (`jurisdiction:{state}:…`,
//!   `meet:{source}:{meet_id}:…`), so a retry resumes the same run and a replayed sweep cannot mint a
//!   second one. Deterministic by construction: the digest is a function of the parts, never of time.
//! * [`verify`] — deterministic sampling and row-level comparison of the workbook's data sheets
//!   against the store's canonical rows, which is what closes the gap the seal leaves: the seal proves
//!   the meta sheets carry the store's counts, this proves the data sheets carry its entities.
//!
//! Both lanes read the store and the domain and name nothing back, so a projection's verdict cannot
//! depend on the service that published it.

#![forbid(unsafe_code)]

pub mod identity;
pub mod verify;

#[cfg(test)]
#[path = "verify_tests.rs"]
mod verify_tests;
