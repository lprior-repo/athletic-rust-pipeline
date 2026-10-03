pub mod collect;
pub mod map;
pub mod parse;

use census_domain::UsJurisdiction;

pub const SOURCE_ID: &str = "arbiter_orgs";
pub const ASSOCIATION: &str = "Arbiter";
pub const HOST: &str = "https://services.arbitersports.com";
pub const TOKEN_URL: &str = "https://token.arbitersports.com/connect/token";
pub const DIRECTORY_URL: &str = "https://live.arbiter.io/directory/";
pub const TOKEN_SCOPE: &str = "Registration";
pub const PAGE_SIZE: u64 = 200;
pub const MAX_PAGES: u64 = 64;

const ORGS: [(UsJurisdiction, &str); 4] = [
    (UsJurisdiction::NewHampshire, "2132"),
    (UsJurisdiction::Kentucky, "2507"),
    (UsJurisdiction::Montana, "4497"),
    (UsJurisdiction::WestVirginia, "4223"),
];

pub fn org_for(state: UsJurisdiction) -> Option<&'static str> {
    ORGS.iter()
        .find(|(covered, _)| *covered == state)
        .map(|(_, org)| *org)
}

pub fn covered_states() -> impl Iterator<Item = UsJurisdiction> {
    ORGS.iter().map(|(state, _)| *state)
}

pub use collect::{collect, Options};

#[cfg(test)]
mod tests;
