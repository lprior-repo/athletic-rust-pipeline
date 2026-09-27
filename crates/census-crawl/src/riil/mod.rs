use census_domain::UsJurisdiction;

pub const HOST: &str = "https://riil.org";
pub const SOURCE_ID: &str = "riil_directory";
pub const ASSOCIATION: &str = "riil";
pub const STATE: UsJurisdiction = UsJurisdiction::RhodeIsland;

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
}

mod collect;
mod map;
mod pages;

pub use collect::collect;
pub use map::{school_entities, SchoolExtract};
pub use pages::{parse_directory, parse_sport_label};

#[cfg(test)]
mod tests;
