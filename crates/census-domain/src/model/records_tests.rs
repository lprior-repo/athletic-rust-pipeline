//! The test cases for the records subtree, one file per group of rows.
//!
//! The twin lives beside its subject under `records_tests/` for the same reason the records did:
//! the source module is a re-export surface, and the cases that pin what a row means are long enough
//! that keeping them in one file would put the file over the length this repository allows.

#[path = "records_tests/joins.rs"]
mod joins;
#[path = "records_tests/measure.rs"]
mod measure;
#[path = "records_tests/observations.rs"]
mod observations;
#[path = "records_tests/queue.rs"]
mod queue;
