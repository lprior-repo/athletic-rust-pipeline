use census_domain::UsJurisdiction;

pub const HOST: &str = "https://mpa.cc";
pub const SOURCE_ID: &str = "mpa_directory";
pub const ASSOCIATION: &str = "mpa";
pub const STATE: UsJurisdiction = UsJurisdiction::Maine;

pub mod collect;
pub mod map;
pub mod pages;

pub use collect::collect;
pub use map::school_entities;
pub use pages::{parse_directory, parse_staff_table};

pub const HOST_WWW: &str = "https://www.mpa.cc";
const DIRECTORY_PATH: &str = "/SchoolPages/School.aspx";
const STAFF_PATH: &str = "/SchoolPages/School.aspx";

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
