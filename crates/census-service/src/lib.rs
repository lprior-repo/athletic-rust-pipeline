#![forbid(unsafe_code)]

#[cfg(test)]
#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

pub mod bootstrap;
pub mod census;
pub mod coachverify;
pub mod ingress;
pub mod outcome;
pub mod restate_services;
pub mod school_address;
pub mod spawn;

pub use census::{collect_milesplit, consolidate, CollectOptions, CollectReport};
