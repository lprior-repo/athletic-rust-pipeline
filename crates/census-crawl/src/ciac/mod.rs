pub mod collect;
pub mod map;
pub mod pages;
mod search;
use census_domain::UsJurisdiction;

pub use collect::collect;
pub use map::{school_entities, SchoolExtract, SchoolTable};
pub use pages::{parse_directory, parse_gender, parse_sport_label};

pub const HOST: &str = "https://ciac.fpsports.org";
pub const SOURCE_ID: &str = "ciac_directory";
pub const ASSOCIATION: &str = "ciac";
pub const STATE: UsJurisdiction = UsJurisdiction::Connecticut;

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
