//! CIAC sports directory adapter (Connecticut).
//!
//! Source: `https://ciacsports.com/` — the Connecticut Interscholastic Athletic Conference
//! publishes one state directory page in which every school is its own `<table>`: the school's
//! name plus one row per sport the school offers, each row naming the head coach for that side.
//! Cross country and track and field appear as their own row labels (Boys/Girls Cross Country,
//! Boys/Girls Indoor and Outdoor Track), which is the role this adapter exists for.
//!
//! What the page carries: school name, sport label, head-coach name. It publishes no coach email
//! and no athletic-director email, so this adapter mints coach identities without contacts and
//! never invents one.
//!
//! Deliberately ignored even where the page exposes them: street address, phone, fax, principal,
//! superintendent, and every athlete field.
//!
//! Measured yield (research lane `coach-directories-national`): 182 schools carry an XC/TF
//! head-coach row across 1043 published rows.
//!
//! Request cost: one GET of the directory page per run. The host publishes no rate limit in its
//! `robots.txt`; the transport's per-host floor still applies.

pub mod collect;
pub mod map;
pub mod pages;
mod search;
use census_domain::UsJurisdiction;

pub use collect::collect;
pub use map::{school_entities, SchoolExtract, SchoolTable};
pub use pages::{parse_directory, parse_gender, parse_sport_label};

/// Host serving the CIAC directory.
pub const HOST: &str = "https://ciacsports.com";
/// Evidence/adapter slug used in [`SourceRef`](census_domain::model::SourceRef)s.
pub const SOURCE_ID: &str = "ciac_directory";
/// Association slug carried by every school identity this adapter mints.
pub const ASSOCIATION: &str = "ciac";
/// Jurisdiction this adapter covers.
pub const STATE: UsJurisdiction = UsJurisdiction::Connecticut;

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
