pub mod map;
pub mod parse;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use census_domain::UsJurisdiction;

pub use map::school_entities;
pub use map::ProfileFacts;
pub use map::SchoolExtract;
pub use parse::parse_coaches_from_profile;
pub use parse::parse_directory_links;
pub use parse::parse_school_profile;

pub const HOST: &str = "https://uhsaa.org";
pub const SOURCE_ID: &str = "uhsaa";
pub const ASSOCIATION: &str = "uhsaa";
pub const STATE: UsJurisdiction = UsJurisdiction::Utah;

mod collect;
pub use collect::collect;

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}
