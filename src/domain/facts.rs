//! Validated source facts. The display-name, numeric and location newtypes live
//! in the child modules of the same name; `text_validation` holds the shared
//! text check.

mod location;
mod names;
mod scalars;
mod text_validation;

pub use self::location::Location;
pub use self::names::{AthleteName, CityName, RegionName, SchoolName};
pub use self::scalars::{ConfidenceScore, GraduationYear, RetryCount};

#[cfg(test)]
mod facts_contract_tests;
