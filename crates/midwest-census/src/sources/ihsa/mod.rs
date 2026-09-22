//! IHSA API adapter (Illinois)
//!
//! The IHSA exposes two endpoints for this mission:
//!
//! * `GET https://api.ihsa.org/v1/schools` — one request returns all 828 member schools with
//!   `SchoolID` (zero-padded 4-char string like `"0101"`), `nameFormal`, `city`, and more.
//! * `GET https://api.ihsa.org/v1/schools/{SchoolID}/staff2` — per-school staff grouped by
//!   category (`Administration`, `Boys Athletics - Head Coaches`, etc.), each row carrying
//!   `PersonID`, `Name` (with optional honorific), `DefaultTitle` (e.g. `"Boys Cross Country Head Coach"`),
//!   `RoleID` (`HCB-CCB`, `HCG-TRG`, `C1-BoysAD`, `D1-GirlsAD`, etc.), and `email`.
//!
//! # Fields observed
//! Schools: `SchoolID`, `nameFormal`, `city`, `State`, `membershipType`, `type`, `URL`. The row is
//! always an Illinois school, so the `State` field is not read.
//! Staff: `PersonID`, `Name`, `DefaultTitle`, `RoleID`, `email`.
//!
//! # Deliberately ignored fields (never read, never stored)
//! `Phone`, `Fax`, `Address`, `POBox`, `Zip`, `Latitude`, `Longitude`, `Color1/2`,
//! `schoolLogo`, `isCPS`, `DirectorySort` — plus any non-coaching office role data
//! (secretary, administrative assistant, athletic trainer, principal, superintendent,
//! business manager, tech director, custodian). Those columns are not part of the schema.

mod collect;
mod map;
mod parse;
mod staff;

pub use collect::collect;
pub use map::{parse_coach, parse_school, reveal_address_for};
pub use parse::{
    parse_email, parse_schools, parse_staff, SchoolRecord, SchoolsEnvelope, StaffPerson,
};
pub use staff::{parse_coach_title, parse_role, strip_honorific};

/// API base URL for the IHSA.
const IHSA_API: &str = "https://api.ihsa.org";

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

// The jurisdiction this adapter covers: the IHSA is the Illinois association.
use census_domain::UsJurisdiction;

// The moved test module reaches these through `use super::*`; no production path in this
// file needs them, so they are bound for tests only instead of widening their visibility.
#[cfg(test)]
use census_domain::model::{
    normalize_name, CanonicalSchool, CoachRole, Gender, SourceNamespace, Sport,
};

#[cfg(test)]
mod tests;
