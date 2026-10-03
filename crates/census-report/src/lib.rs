#![forbid(unsafe_code)]

#[cfg(test)]
#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

pub mod bests;
mod csv_safety;
pub mod export;
pub mod report;
pub mod workbook;
