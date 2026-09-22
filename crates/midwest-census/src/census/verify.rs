//! Deterministic sampling and row-level verification against the store.
//!
//! The seal proves that the workbook's meta sheets (Coverage, Run Metrics) carry the counts the
//! store reports. This module closes the gap: it reads the data sheets and checks that sampled
//! rows correspond to entities the store actually holds.

mod columns;
mod compare;
mod sample;

pub use columns::{
    column_index, missing_columns, sheets_matching_prefix, ATHLETES_REQUIRED, PERFORMANCES_REQUIRED,
};
pub use compare::{verify_athletes, verify_performances, Discrepancy, EntityCheck};
pub use sample::sample_indices;
