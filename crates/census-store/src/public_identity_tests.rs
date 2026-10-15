use crate::{Store, StoreError, StoreResult, Table};

mod authority_edges;
mod corruption;
mod counter_corruption;
mod enrichment_refusal;
mod ledger;
mod lifecycle;
mod lineage_guards;
mod lineage_namespace_edges;
mod projection_guards;
mod recovery;
mod review_authority;
mod revision_races;
mod snapshot;
mod source_isolation;
mod support;
mod transactions;
mod write_guards;

use support::{fixture, physical_state, retained_state, subject, TestResult};
