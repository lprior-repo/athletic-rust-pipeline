//! Maine Principals' Association directory adapter (Maine).
//!
//! Source: `https://mpa.cc/` — the Maine Principals' Association publishes one state directory
//! page in which every school is its own `<table>`: the school's name plus one row per sport the
//! school offers, each row naming the head coach for that side. Cross country and track and field
//! appear as their own row labels (Boys/Girls Cross Country, Boys/Girls Indoor and Outdoor Track),
//! which is the role this adapter exists for.
//!
//! What the page carries: school name, sport label, head-coach name. It publishes no coach email
//! and no athletic-director email, so this adapter mints coach identities without contacts and
//! never invents one.
//!
//! Deliberately ignored even where the page exposes them: street address, phone, fax, principal,
//! superintendent, and every athlete field.
//!
//! Measured yield (research lane `coach-directories-national`): 81 schools carry an XC/TF
//! head-coach row across 310 published rows.
//!
//! Request cost: one GET of the directory page per run.

use census_domain::UsJurisdiction;

/// Host serving the MPA directory.
pub const HOST: &str = "https://mpa.cc";
/// Evidence/adapter slug used in [`SourceRef`](census_domain::model::SourceRef)s.
pub const SOURCE_ID: &str = "mpa_directory";
/// Association slug carried by every school identity this adapter mints.
pub const ASSOCIATION: &str = "mpa";
/// Jurisdiction this adapter covers.
pub const STATE: UsJurisdiction = UsJurisdiction::Maine;

pub mod collect;
pub mod map;
pub mod pages;

pub use collect::collect;
pub use map::school_entities;
pub use pages::{parse_directory, parse_staff_table};

/// Host serving the MPA directory (www subdomain).
pub const HOST_WWW: &str = "https://www.mpa.cc";
/// Directory page path.
const DIRECTORY_PATH: &str = "/SchoolPages/School.aspx";
/// Staff page path (takes `SchoolID` and `tab` query params).
const STAFF_PATH: &str = "/SchoolPages/School.aspx";

/// Adapter options.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Stop after this many schools (smoke runs).
    pub limit: Option<usize>,
    /// Ignore cached HTTP bodies and re-fetch.
    pub refresh: bool,
    /// ISO date stamped into evidence.
    pub observed_on: String,
    /// Restrict to these jurisdictions when the provider spans several states.
    pub states: Vec<UsJurisdiction>,
    /// School names to resolve when the provider has no bulk index.
    pub school_names: Vec<String>,
}

#[cfg(test)]
mod tests;
