//! `census-report` — the reporting plane (§17): coverage, bests, PR projection, workbook export.
//!
//! Layers:
//!
//! * [`report`] — measured census output (`report.json`, per-state CSV).
//! * [`bests`] — per-athlete best marks reduced from the consolidated performance table.
//! * [`workbook`] — the census as one spreadsheet.
//!
//! The plane is read-only over [`census_store`]: it projects what the store already holds and never
//! acquires, reconciles or writes canonical rows.

#![forbid(unsafe_code)]

pub mod bests;
pub mod report;
pub mod workbook;
