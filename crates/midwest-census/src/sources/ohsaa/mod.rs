//! OHSAA officials portal adapter (Ohio)
//!
//! The OHSAA myOHSAA portal on `officials.myohsaa.org` provides:
//!
//! * **School search** — `GET /Outside/SearchSchool?Name=<query>` returns an HTML table with
//!   one `<tr>` per matching school carrying the ALL-CAPS name, city, and the `ohsaaId` link.
//!   Search is prefix-based; a name query such as `Name=Mason` returns exact matches plus
//!   prefixes, each repeated many times (the same `ohsaaId` appears in every row for a given
//!   school). Duplicates must be deduplicated by `ohsaaId`.
//! * **Sports information** — `GET /Outside/Schedule/SportsInformation?ohsaaId=<id>` returns a
//!   table with columns `Sport | Head Boys Coach | Head Girls Coach`. Each cell carries either
//!   `N/A` (sport not offered), `TBA (Div-X)` (coach not yet appointed), or a coach name like
//!   `Joe DePalma (Div-I)` wrapped in a `mailto:` anchor with the coach email.
//! * **Athletic department** — `GET /Outside/Schedule/AthleticDirector?ohsaaId=<id>` returns the
//!   athletic director name and email, plus assistant/secretary rows that must be excluded.
//!
//! # Fields observed
//! Search: ALL-CAPS school name, city, `ohsaaId` from the `View` link.
//! Sports: sport label, coach name (may carry honorific "Coach"), email from `mailto:` href.
//! AD: director name, email. Office roles (assistant AD, secretary) are present but excluded.
//!
//! # Deliberately ignored fields (never read, never stored)
//! Street address, phone, fax, building number, assistant athletic director, assistant athletic
//! secretary, athletic trainer, principal, superintendent, business manager, and any athlete data.
//!
//! These columns exist on the page but are not part of the canonical schema.

pub mod collect;
pub mod map;
pub mod pages;
pub mod parse;

pub use collect::collect;
pub use map::{school_entities, AdPage, CoachEntry, SchoolExtract, SearchResult};
pub use pages::{parse_ad_page, parse_coach_cell, parse_sport_label, parse_sports_table};
pub use parse::{parse_search, resolve_school_name, strip_honorific};

/// Host serving the OHSAA officials portal.
pub const HOST: &str = "https://officials.myohsaa.org";
/// Evidence/adapter slug used in [`SourceRef`](census_domain::model::SourceRef)s.
pub const SOURCE_ID: &str = "ohsaa_portal";
/// Association slug carried by every school identity this adapter mints.
pub const ASSOCIATION: &str = "ohsaa";
/// State code for Ohio.
pub const STATE: &str = "OH";

/// URL patterns.
const SEARCH_PATH: &str = "/Outside/SearchSchool";
const SPORTS_PATH: &str = "/Outside/Schedule/SportsInformation";
const AD_PATH: &str = "/Outside/Schedule/AthleticDirector";
const SCHOOL_INFO_PATH: &str = "/Outside/Schedule";

/// Adapter options.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Stop after this many schools (smoke runs).
    pub limit: Option<usize>,
    /// Ignore cached HTTP bodies and re-fetch.
    pub refresh: bool,
    /// ISO date stamped into evidence.
    pub observed_on: String,
    /// Restrict to these state codes when the provider spans several states.
    pub states: Vec<String>,
    /// School names to resolve when the provider has no bulk index.
    pub school_names: Vec<String>,
}

#[cfg(test)]
mod tests;
