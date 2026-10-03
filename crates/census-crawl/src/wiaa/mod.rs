use crate::net::FetchOptions;
use crate::AdapterContext;
use census_domain::UsJurisdiction;

pub const HOST: &str = "https://schools.wiaawi.org";
pub const SOURCE_ID: &str = "wiaa_directory";
pub const ASSOCIATION: &str = "wiaa";

const INDEX_PATH: &str = "/Directory/School/DirectoryLetter";
const SCHOOL_PATH: &str = "/Directory/School/GetDirectorySchool";
const LETTERS: [char; 26] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

mod collect;
mod map;
mod parse;
mod primitives;

pub use collect::collect;
pub use map::{
    parse_admin_role, parse_coach_role, parse_sport_label, school_entities, strip_honorific,
    SchoolExtract,
};
pub use parse::{
    parse_directory_letter, parse_enrollment, parse_school_page, CoachRow, IndexEntry, SchoolPage,
    StaffRow,
};
pub use primitives::decode_cfemail;

fn letters_for(school_names: &[String]) -> Vec<char> {
    if school_names.is_empty() {
        return LETTERS.to_vec();
    }
    let mut letters: Vec<char> = school_names
        .iter()
        .filter_map(|name| {
            name.chars()
                .find(char::is_ascii_alphabetic)
                .map(|ch| ch.to_ascii_uppercase())
        })
        .collect();
    letters.sort_unstable();
    letters.dedup();
    letters
}

fn fetch_options(ctx: &AdapterContext<'_>, options: &Options) -> FetchOptions {
    FetchOptions {
        refresh: options.refresh || ctx.refresh,
        ..ctx.fetch_options()
    }
}

fn count(value: usize) -> u64 {
    u64::try_from(value).map_or(u64::MAX, |value| value)
}

#[cfg(test)]
mod tests;
