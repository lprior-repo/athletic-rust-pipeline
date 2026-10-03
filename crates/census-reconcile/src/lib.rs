#![forbid(unsafe_code)]

#[cfg(test)]
#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

pub mod identity;
pub mod index;
pub mod verify;

#[cfg(test)]
#[path = "verify_tests.rs"]
mod verify_tests;
