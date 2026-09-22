//! MileSplit HTML adapter — the 12-state discovery layer.
//!
//! Only robots-permitted, server-rendered surfaces are used:
//!
//! * `/{teams}` — the per-state team index (one row per team: name, location, team id)
//! * `/{teams}/<id>-<slug>/roster` — the graded roster table
//! * `/{athletes}/<id>-<slug>` — public athlete profile (used only for confirmation, never required)
//!
//! `/api/`, `/rankings`, `/virtual-meets` and `/contact` are robots-disallowed and are never
//! requested; the roster HTML carries the same graduating-year evidence the JSON API would provide.

mod fetch;
mod normalize;
mod parse;
mod wire;

pub use fetch::{fetch_roster, fetch_team_index};
pub use normalize::roster_entities;
pub use parse::{parse_roster, parse_team_index};
pub use wire::{Roster, RosterAthlete, Site, TeamRef};

// The moved bodies reach these through `super::*`; the test module is the only reader.
#[cfg(test)]
use census_domain::model::{CanonicalAthlete, Gender, GradYear, SchoolYear, Sport};

#[cfg(test)]
mod tests;
