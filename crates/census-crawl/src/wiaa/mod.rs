//! WIAA school directory adapter (Wisconsin)
//!
//! The Wisconsin Interscholastic Athletic Association runs a public school/team/coach database on
//! `schools.wiaawi.org` (IIS + ASP.NET MVC 5.2, no `robots.txt` published — `GET /robots.txt`
//! answers HTTP 404). Two endpoints carry everything this adapter needs, both verified live:
//!
//! * **Index** — `GET /Directory/School/DirectoryLetter?LetterBtn=<A..Z>` returns an HTML fragment
//!   whose `#tblSchools` table lists every school starting with that letter, one `<tr>` per school,
//!   linking `/Directory/School/GetDirectorySchool?orgID=<OrganizationID>`. `LetterBtn=-1` (the UI's
//!   "all" button) returns an empty 3 KiB fragment, so 26 letter requests are the only way to
//!   enumerate the directory; the 26 letters returned 629 distinct orgIDs (600 High School, 29
//!   Middle School) when the directory was captured on 2026-09-19.
//! * **School** — `GET /Directory/School/GetDirectorySchool?orgID=<OrganizationID>` returns the full
//!   school page: identity block (`Level`, `Class`, `City`, `Conference (Default)`,
//!   `Enrollment (<school year>)`, `Website` button), the `#tblAdminList` administration table
//!   (`Role | Name | Email`) and the `#tblCoachList` head-coach table
//!   (`Sport | Name | Role | Email`). One request per school; the full sweep is ~629 polite
//!   requests (~113 MB).
//!
//! # Fields observed
//! Index row: the `title` attribute (full school name — the visible `<h5>` is CSS-truncated for long
//! names), `Level`, `City`, and the `orgID` link. School page: `Level`, `City`,
//! `Conference (Default)`, `Enrollment (…)` → `School:`, the `Website` button href, admin rows
//! (`Role`, `Name`, `Email`) and coach rows (`Sport`, `Name`, `Role`, `Email`).
//!
//! # Deliberately ignored fields (never read, never stored)
//! The school page also publishes the school's street address, ZIP, switchboard and fax; none of
//! those are part of the canonical schema, so they are not parsed at all. No person-level phone
//! number or home address exists on this surface, and no athlete contact data exists in this source
//! to begin with. Coach-table rows for sports other than TF/XC contribute nothing but a counter.
//!
//! # Emails
//! Coach and administrator emails are published role-scoped but Cloudflare-obfuscated in the HTML
//! (`<span class="__cf_email__" data-cfemail="<hex>">`); the page's own `email-decode.min.js` renders
//! them in plaintext for any anonymous visitor, and this adapter decodes the same encoding (first
//! byte = XOR key). Decoded values are only accepted when they parse as an address, and nothing is
//! ever derived from a name. The measured fill rate for the captured pages is asserted in the tests.

use crate::net::FetchOptions;
use crate::AdapterContext;
use census_domain::UsJurisdiction;

/// Host serving the WIAA school directory.
pub const HOST: &str = "https://schools.wiaawi.org";
/// Evidence/adapter slug used in [`SourceRef`](census_domain::model::SourceRef)s.
pub const SOURCE_ID: &str = "wiaa_directory";
/// Association slug carried by every school identity this adapter mints.
pub const ASSOCIATION: &str = "wiaa";

const INDEX_PATH: &str = "/Directory/School/DirectoryLetter";
const SCHOOL_PATH: &str = "/Directory/School/GetDirectorySchool";
/// Directory letters. `-1` is the UI's "all" button and returns an empty fragment.
const LETTERS: [char; 26] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z',
];

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

mod collect;
mod map;
mod parse;
mod primitives;

pub use collect::collect;
pub use map::{
    parse_admin_role, parse_coach_role, parse_sport_label, school_entities, strip_honorific,
    SchoolExtract,
};
pub use parse::{
    parse_directory_letter, parse_enrollment, parse_school_page, CoachRow, IndexEntry, SchoolPage,
    StaffRow,
};
pub use primitives::decode_cfemail;

/// Letters worth walking: every letter, or only the distinct first letters of the requested names
/// when the caller asked for specific schools.
fn letters_for(school_names: &[String]) -> Vec<char> {
    if school_names.is_empty() {
        return LETTERS.to_vec();
    }
    let mut letters: Vec<char> = school_names
        .iter()
        .filter_map(|name| {
            name.chars()
                .find(char::is_ascii_alphabetic)
                .map(|ch| ch.to_ascii_uppercase())
        })
        .collect();
    letters.sort_unstable();
    letters.dedup();
    letters
}

/// Fetch options for one request. `options.refresh` and the run-level `ctx.refresh` both ask for a
/// cache bypass, so either flag wins.
fn fetch_options(ctx: &AdapterContext<'_>, options: &Options) -> FetchOptions {
    FetchOptions {
        refresh: options.refresh || ctx.refresh,
        ..ctx.fetch_options()
    }
}

fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

// -------------------------------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests;
