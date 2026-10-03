pub const SOURCE_ID: &str = "tssaa";
pub const DIRECTORY_URL: &str = "https://portal.tssaa.org/common/directory/";

mod collect;

mod fields;
mod map;
mod parse;
mod postal;
mod staff;
mod staff_year;

pub use collect::{collect, Options};
pub use parse::{parse_school_list, parse_school_page, CoachRow, SchoolRead};

#[cfg(test)]
mod tests;
