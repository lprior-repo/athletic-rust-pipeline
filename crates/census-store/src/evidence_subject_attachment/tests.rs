use super::*;

mod legacy_read_bounds;
mod namespace;
mod protected_pruning;
mod protected_routes;
mod workflow;

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
