//! KSHSAA directory adapter (Kansas)
//!
//! The KSHSAA directory API exposes one endpoint that returns every member school with its athletic
//! director's name and email:
//! `GET https://kshsaa-api.kshsaa.org/directory/search/name/a/`
//!
//! Per-letter queries (`/directory/search/name/<letter>/`) are supported but the single `a` request
//! already yields the full ~526-school universe.
//!
//! # Fields observed
//! `Id`, `Identifier` (e.g. `KSS0307`), `SchoolName`, `MailingCity`, `Class`, `Enrollment`,
//! `WebSite`, `ADName`, `ADEmail`.
//!
//! # Deliberately ignored fields (never read, never stored)
//! `ADCell`, `PresCell`, `PrincipalCell`, `PrincipalName`, `PresName`, `PresEmail`, `SchoolPhone`,
//! `SchoolFax`, `Email`, `TwitterUserName` — any phone field, any home or cell number, and any
//! non-coaching office role data. Those columns are not part of the schema.

mod collect;
mod parse;
mod wire;

pub use collect::{collect, Options};
pub use parse::{parse_ad_coach, parse_school};
pub use wire::{parse_records, KshsaaRecord};

/// Association slug carried by every school identity this adapter mints, and the association whose
/// observation rows are filed under it.
pub const ASSOCIATION: &str = "kshsaa";

#[cfg(test)]
use census_domain::model::{normalize_name, CanonicalSchool, CoachRole, Gender, SourceNamespace};

#[cfg(test)]
mod tests;
