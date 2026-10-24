use super::*;

mod binding;
mod corruption;
mod evidence;
pub(super) mod fixture;
mod graph_records;
mod identity;
mod matched_group;
mod membership;
mod request_order;
mod revocation;
mod split;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
