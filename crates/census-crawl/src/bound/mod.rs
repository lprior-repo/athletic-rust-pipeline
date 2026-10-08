pub mod collect;
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
pub use parse::parse;
pub use parse::parse_gender;
pub use parse::parse_sport_label;
pub use parse::CoachRow;
pub use parse::ParseResult;

pub const HOST: &str = "https://www.gobound.com";
pub const SOURCE_ID: &str = "bound";
pub const ASSOCIATION: &str = "bound";
pub const STATE: UsJurisdiction = UsJurisdiction::Iowa;

pub fn school_namespace() -> census_domain::model::SourceNamespace {
    census_domain::model::SourceNamespace::AssociationSchool {
        association: ASSOCIATION.to_string(),
    }
}
