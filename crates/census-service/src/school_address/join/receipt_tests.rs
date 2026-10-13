use super::*;
use census_domain::model::serialized_digest;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

#[path = "receipt_tests/budget.rs"]
mod budget;
#[path = "receipt_tests/fixture.rs"]
mod fixture;
#[path = "receipt_tests/physical.rs"]
mod physical;
#[path = "receipt_tests/placement.rs"]
mod placement;
#[path = "receipt_tests/refusal.rs"]
mod refusal;
#[path = "receipt_tests/retained.rs"]
mod retained;
#[path = "receipt_tests/review.rs"]
mod review;
#[path = "receipt_tests/scope_budget.rs"]
mod scope_budget;
#[path = "receipt_tests/witness.rs"]
mod witness;

const WITNESS_PHASE: &str = "school-address-join-application-v1";
const APPLICATION_VERSION: &str = "school-address-join-application/v1";
const OPERATION_PREFIX: &str = "school-address-join:application:v1:";

#[path = "receipt_tests/partial_target.rs"]
mod partial_target;

#[path = "receipt_tests/merge_boundary.rs"]
mod merge_boundary;
#[path = "receipt_tests/revision_interleaving.rs"]
mod revision_interleaving;
