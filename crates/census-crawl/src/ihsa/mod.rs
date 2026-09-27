
mod collect;
mod map;
mod parse;
mod staff;
pub mod tournament;

pub use collect::collect;
pub use map::{parse_coach, parse_school};
pub use parse::{
    parse_email, parse_schools, parse_staff, SchoolRecord, SchoolsEnvelope, StaffPerson,
};
pub use staff::{parse_coach_title, parse_role, strip_honorific};

const IHSA_API: &str = "https://api.ihsa.org";

pub const ASSOCIATION: &str = "ihsa";

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

use census_domain::UsJurisdiction;

#[cfg(test)]
use census_domain::model::{
    normalize_name, CanonicalSchool, CoachRole, Gender, SourceNamespace, Sport,
};

#[cfg(test)]
mod tests;
