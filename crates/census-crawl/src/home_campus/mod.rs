pub mod collect;
mod entities;
pub mod map;
pub mod parse;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

use census_domain::UsJurisdiction;

pub use collect::collect;
pub use collect::Options;
pub use map::school_entities;
pub use map::ProfileFacts;
pub use map::SchoolExtract;
pub use parse::parse_directory_links;
pub use parse::parse_school_details;
pub use parse::parse_sport_and_gender;

pub const HOST: &str = "https://www.cifsshome.org";
pub const SOURCE_ID: &str = "home_campus";
pub const ASSOCIATION_CA: &str = "cif";
pub const ASSOCIATION_FL: &str = "fhsaa";
pub const ASSOCIATION_NJ: &str = "njsiaa";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Section {
    pub number: u32,
    pub state: UsJurisdiction,
    pub association: &'static str,
}

pub const SECTIONS: [Section; 12] = [
    Section {
        number: 1,
        state: UsJurisdiction::California,
        association: ASSOCIATION_CA,
    },
    Section {
        number: 2,
        state: UsJurisdiction::California,
        association: ASSOCIATION_CA,
    },
    Section {
        number: 3,
        state: UsJurisdiction::California,
        association: ASSOCIATION_CA,
    },
    Section {
        number: 4,
        state: UsJurisdiction::California,
        association: ASSOCIATION_CA,
    },
    Section {
        number: 5,
        state: UsJurisdiction::California,
        association: ASSOCIATION_CA,
    },
    Section {
        number: 6,
        state: UsJurisdiction::California,
        association: ASSOCIATION_CA,
    },
    Section {
        number: 7,
        state: UsJurisdiction::California,
        association: ASSOCIATION_CA,
    },
    Section {
        number: 8,
        state: UsJurisdiction::California,
        association: ASSOCIATION_CA,
    },
    Section {
        number: 9,
        state: UsJurisdiction::California,
        association: ASSOCIATION_CA,
    },
    Section {
        number: 13,
        state: UsJurisdiction::California,
        association: ASSOCIATION_CA,
    },
    Section {
        number: 10,
        state: UsJurisdiction::Florida,
        association: ASSOCIATION_FL,
    },
    Section {
        number: 12,
        state: UsJurisdiction::NewJersey,
        association: ASSOCIATION_NJ,
    },
];
