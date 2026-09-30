pub const SOURCE_ID: &str = "tssaa";

mod fields;
mod parse;

pub use parse::{parse_school_list, parse_school_page, CoachRow, SchoolRead};
