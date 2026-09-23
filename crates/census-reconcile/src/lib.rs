//! `census-reconcile` — the reconciliation lane (§17): the normalisation that derives the store's
//! durable indexes, deterministic workflow identity, and the row-level check that holds the published
//! workbook against the store.
//!
//! * [`identity`] — every workflow's address derived from explicit values (`jurisdiction:{state}:…`,
//!   `meet:{source}:{meet_id}:…`), so a retry resumes the same run and a replayed sweep cannot mint a
//!   second one. Deterministic by construction: the digest is a function of the parts, never of time.
//! * [`index`] — the normalisation pass that turns the store's merged rows into its derived plane:
//!   source-object identities, the retained conflict and review queues, per-jurisdiction coverage and
//!   one snapshot per pass. It reuses the code that renders those values, so a reader of the store and
//!   a reader of the workbook cannot be shown different findings.
//! * [`verify`] — deterministic sampling and row-level comparison of the workbook's data sheets
//!   against the store's canonical rows, which is what closes the gap the seal leaves: the seal proves
//!   the meta sheets carry the store's counts, this proves the data sheets carry its entities.
//!
//! All three lanes read the store and the domain and name nothing back up the graph, so a projection's
//! verdict cannot depend on the service that published it.

#![forbid(unsafe_code)]

pub mod identity;
pub mod index;
pub mod verify;

#[cfg(test)]
#[path = "verify_tests.rs"]
mod verify_tests;
