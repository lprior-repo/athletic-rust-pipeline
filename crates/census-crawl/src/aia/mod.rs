use census_domain::UsJurisdiction;

pub mod collect;
pub mod map;
pub mod parse;

#[cfg(test)]
mod tests;

pub use collect::collect;
pub use map::{school_entities, SchoolExtract};
pub use parse::{
    parse_gender, parse_school_profile, parse_search_json, parse_sport_label, CoachRow, ParsedRow,
    SchoolInfo, SchoolProfile,
};

pub const HOST: &str = "https://aiaonline.org";
pub const SOURCE_ID: &str = "aia";
pub const ASSOCIATION: &str = "aia";
pub const STATE: UsJurisdiction = UsJurisdiction::Arizona;

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}
