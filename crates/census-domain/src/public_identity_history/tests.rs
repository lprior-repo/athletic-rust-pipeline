use super::*;

mod binding;
mod corruption;
mod evidence;
mod fixture;
mod graph_records;
mod identity;
mod matched_group;
mod membership;
mod revocation;
mod split;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
