
#![forbid(unsafe_code)]

pub mod identity;
pub mod index;
pub mod verify;

#[cfg(test)]
#[path = "verify_tests.rs"]
mod verify_tests;
