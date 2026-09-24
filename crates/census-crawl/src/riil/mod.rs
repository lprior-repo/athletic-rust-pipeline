//! Rhode Island Interscholastic League directory adapter (Rhode Island).
//!
//! Source: `https://riil.org/` — the Rhode Island Interscholastic League publishes one state
//! directory page in which every school is its own `<table>`: the school's name plus one row per
//! sport the school offers, each row naming the head coach for that side. Cross country and track
//! and field appear as their own row labels (Boys/Girls Cross Country, Boys/Girls Indoor and
//! Outdoor Track), which is the role this adapter exists for.
//!
//! What the page carries: school name, sport label, head-coach name. It publishes no coach email
//! and no athletic-director email, so this adapter mints coach identities without contacts and
//! never invents one.
//!
//! Deliberately ignored even where the page exposes them: street address, phone, fax, principal,
//! superintendent, and every athlete field.
//!
//! Measured yield (research lane `coach-directories-national`): 49 schools carry an XC/TF
//! head-coach row across 259 published rows.
//!
//! Request cost: one to two GETs per run (the directory page, and its per-school view where the
//! directory links one).

use census_domain::UsJurisdiction;

/// Host serving the RIIL directory.
pub const HOST: &str = "https://riil.org";
/// Evidence/adapter slug used in [`SourceRef`](census_domain::model::SourceRef)s.
pub const SOURCE_ID: &str = "riil_directory";
/// Association slug carried by every school identity this adapter mints.
pub const ASSOCIATION: &str = "riil";
/// Jurisdiction this adapter covers.
pub const STATE: UsJurisdiction = UsJurisdiction::RhodeIsland;

/// Adapter options.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Stop after this many schools (smoke runs).
    pub limit: Option<usize>,
    /// Ignore cached HTTP bodies and re-fetch.
    pub refresh: bool,
    /// ISO date stamped into evidence.
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
