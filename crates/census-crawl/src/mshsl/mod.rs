//! MSHSL adapter (Minnesota)
//!
//! The MSHSL school view enumerates Minnesota schools; each page publishes the activities director with a
//! Cloudflare-obfuscated email that is decoded client-side.
//!
//! Surfaces used (all robots-permitted: `mshsl.org/robots.txt` disallows only `/core/`, `/profiles/`,
//! `/search*`, `/admin/`, `/user/*` and the `*.pdf|doc|docx|xls|xlsx|csv` extensions, so no PDF is ever
//! requested from here):
//!
//! * `GET /schools` and `/schools?page=N` — the server-rendered school universe (50 rows/page) with the
//!   `/schools/<slug>` page path, school name and locality. The listing is HTML; the `/views/ajax`
//!   endpoint its filters call is not needed and `export/schools.csv` is a robots-disallowed extension.
//! * `GET /schools/<slug>` — school facts (numeric `/group/<id>/` id, classification enrollment, website)
//!   plus the Administration block: `Activities Director` and `Assistant Activities Director` with a
//!   Cloudflare-obfuscated published email that is decoded locally (XOR with the first byte). The same
//!   block publishes office roles (principal, superintendent, AD administrative assistant, trainer,
//!   advisors, Title IX officer, sports representatives): they are parsed and then *rejected*, so no
//!   non-coaching office ever reaches the store.
//! * `GET /jsonapi/views/teams/list_school?views-argument[]=<schoolId>` with a sparse fieldset — the
//!   school's team nodes (`drupal_internal__nid` + path alias), filtered to the track/XC activities.
//! * `GET /api/coaches/<team nid>` — per-team coach records (`name`, `coach_level`, `field_email`).
//!   Records whose `coach_level` is not an MSHSL level (`Non-MSHSL Coach`, `MSHSL Sub-Coach`) are dropped
//!   and every well-formed published address is retained and classified as professional or personal.
//!   `field_work_phone` mixes school extensions with personal mobiles, so the field is not even declared
//!   in the deserializer. [report 09 §"Retention contract"]
//!
//! Every emitted entity carries the URL it came from plus `options.observed_on`; missing fields stay
//! empty and are reported as notes rather than invented.
//!
//! Interface contract (fixed by `sources/mod.rs`): implement `collect` plus public `parse_*` helpers
//! covered by fixture-backed unit tests. This file is owned by a single writer; do not edit any other
//! module while implementing it.
//!
//! Layout: `parse`, `text` and `teams` decode the published payloads, `map` mints canonical
//! entities, and `collect` drives the run and journals it.

mod collect;
mod map;
mod parse;
mod teams;
mod text;

use census_domain::UsJurisdiction;

pub use collect::collect;
pub use map::{
    ad_coaches, ad_role, coach_entities, provider_key, published_coach_email, school_domains,
    school_entities,
};
pub use parse::{
    listing_page_url, parse_admin_entries, parse_next_listing_page, parse_school_detail,
    parse_school_list, school_page_url, AdminEntry, SchoolDetail, SchoolListRow,
};
pub use teams::{
    coach_role, is_published_level, parse_coach_records, parse_team_nodes, select_team_nodes,
    team_sport, CoachRecord, TeamCoaches, TeamNode,
};
pub use text::{decode_cfemail, decode_cfemail_fragment, strip_honorific};

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

/// Source id used in evidence and journal notes.
const SOURCE_ID: &str = "mshsl";
/// Namespace for the provider's own coach key space (MSHSL publishes no coach id).
const COACH_NAMESPACE: &str = "mshsl_team_coach";
/// Public school listing (page 0; `?page=N` for the rest).
pub const SCHOOL_LIST_URL: &str = "https://www.mshsl.org/schools";
/// School page prefix: `SCHOOL_URL_PREFIX + slug`.
pub const SCHOOL_URL_PREFIX: &str = "https://www.mshsl.org/schools/";
/// JSON:API view listing one school's team nodes (the source of the coach endpoint's nids).
pub const TEAMS_VIEW_URL: &str = "https://www.mshsl.org/jsonapi/views/teams/list_school";
/// Per-team coach endpoint prefix: `COACH_API_PREFIX + nid`.
pub const COACH_API_PREFIX: &str = "https://www.mshsl.org/api/coaches/";
/// Bound on listing pages walked in one run (Power-of-Ten rule 2).
const MAX_LISTING_PAGES: usize = 64;
/// Track/XC teams per school: 2 sports × 2 genders.
const MAX_TEAMS_PER_SCHOOL: usize = 4;
/// Guard against a pathological obfuscated payload.
const MAX_CFEMAIL_HEX: usize = 512;

#[cfg(test)]
mod tests;
