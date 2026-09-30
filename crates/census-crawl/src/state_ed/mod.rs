pub const SOURCE_ID: &str = "state_ed";

mod fields;
mod parse;
mod tabular;

pub use parse::{parse_index, parse_profile};
pub use tabular::{parse_tabular, TABULAR_REQUIRED};
