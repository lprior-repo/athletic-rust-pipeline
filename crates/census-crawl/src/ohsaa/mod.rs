
pub mod collect;
pub mod map;
pub mod pages;
pub mod parse;
use census_domain::UsJurisdiction;

pub use collect::collect;
pub use map::{school_entities, AdPage, CoachEntry, SchoolExtract, SearchResult};
pub use pages::{parse_ad_page, parse_coach_cell, parse_sport_label, parse_sports_table};
pub use parse::{parse_search, resolve_school_name, strip_honorific};

pub const HOST: &str = "https://officials.myohsaa.org";
pub const SOURCE_ID: &str = "ohsaa_portal";
pub const ASSOCIATION: &str = "ohsaa";
pub const STATE: UsJurisdiction = UsJurisdiction::Ohio;

const SEARCH_PATH: &str = "/Outside/SearchSchool";
const SPORTS_PATH: &str = "/Outside/Schedule/SportsInformation";
const AD_PATH: &str = "/Outside/Schedule/AthleticDirector";
const SCHOOL_INFO_PATH: &str = "/Outside/Schedule";

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

#[cfg(test)]
mod tests;
