
mod collect;
mod parse;
mod wire;

pub use collect::{collect, Options};
pub use parse::{parse_ad_coach, parse_school};
pub use wire::{parse_records, KshsaaRecord};

pub const ASSOCIATION: &str = "kshsaa";

#[cfg(test)]
use census_domain::model::{normalize_name, CanonicalSchool, CoachRole, Gender, SourceNamespace};

#[cfg(test)]
mod tests;
