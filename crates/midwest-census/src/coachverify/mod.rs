//! The coach-fragment provenance gate: re-derive every shipped coach row from the pages it cites.
//!
//! A coach fragment is a per-state CSV a research lane wrote out of school/association pages. The
//! gate exists because the rows are *claims*: before any of them reaches the store it fetches each
//! row's own `source_url` cells and re-derives the row from the bytes that came back. A row ships
//! only when two independent things hold against those bytes:
//!
//! 1. **value** — at least one of `coach_name` / coach email / `ad_name` / AD email appears
//!    literally; and
//! 2. **role** — the role label is corroborated *next to that value*: the window around the first
//!    occurrence must carry `director` (Athletic Director rows) or `coach` (coach rows). A page-wide
//!    keyword is not enough — a site that mentions a band director elsewhere does not corroborate an
//!    "Athletic Director" row — and a value whose own title reads a *different* role drops the row.
//!
//! Rows that never satisfy both are classified and reported, never relabelled silently:
//! [`Verdict::RoleContradicted`] (the page states another role for that person),
//! [`Verdict::OkRoleContext`] (value present, role only conveyed by page context),
//! [`Verdict::RenderRequired`] (JavaScript shell: no row value is ever in a fetched body),
//! [`Verdict::Mismatch`] (real content, values absent), [`Verdict::Empty`] (nothing came back),
//! [`Verdict::RobotsBlocked`] (the polite fetcher refused: robots.txt disallows every cited page)
//! and [`Verdict::FetchFailed`] (every cited page failed in transport).
//!
//! Fetch passes escalate until both conditions hold. Pass 1 is a plain GET of every cited URL — no
//! early exit, because the role window may sit on a later page than the value. Pass 2 repeats the
//! URLs with `X-Requested-With` + `Referer`, which widget endpoints need. Pass 3 POSTs the NSAA
//! export screen, which answers only to a form POST of the school name. All passes share the
//! fetcher's on-disk cache, so a re-run costs nothing and two fragments citing one page pay for it
//! once.

mod fetch;
mod output;
mod report;
mod verdict;

pub use fetch::{body_text, GateOptions};
pub use output::{fragment_file_name, read_fragment, verify_fragment, write_fragment};
pub use report::{audit_table, cited_hosts, write_audit_csv, write_manifest, write_state_union};
pub use verdict::{
    // compat alias: keep `super::Reconcile` importable as `crate::coachverify::Reconcile`
    reconcile,
    FragmentOutcome,
    Reconcile,
    Reconcile as Reconcile_,
    RowOutcome,
};

use regex::Regex;
use std::sync::LazyLock;

/// The 11 columns a fragment must carry, in the order the importer expects them.
pub const FRAGMENT_COLUMNS: &[&str; 11] = &[
    "school",
    "city",
    "state",
    "sport",
    "role",
    "coach_name",
    "public_professional_email",
    "ad_name",
    "ad_email",
    "source_url",
    "last_observed",
];

/// The verdict column the gate appends (and ignores on input: a verdict is always recomputed).
pub const VERDICT_COLUMN: &str = "verify";

/// Bytes of context kept before the matched value when looking for the role label.
const ROLE_BEFORE: usize = 200;
/// Total bytes of the role window. With [`ROLE_BEFORE`] the window reaches 500 bytes past the value.
const ROLE_WINDOW: usize = 700;
/// Bytes of context kept before the matched value for the contradiction window: tighter, because a
/// title column sits immediately next to a value while a neighbouring staff entry does not.
const CONTRA_BEFORE: usize = 100;
/// Total bytes of the contradiction window.
const CONTRA_WINDOW: usize = 300;

/// The endpoint that answers only to a form POST of the school name.
const NSAA_EXPORT_SCREEN: &str = "direxportscreen.php";

/// School-name suffixes the NSAA query strips before POSTing.
const NSAA_SUFFIXES: &[&str] = &["High School", "HS", "High", "School"];

pub use verdict::Verdict;

/// One fragment row: the 11 columns, with `source_url` split into its citation list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FragmentRow {
    pub school: String,
    pub city: String,
    pub state: String,
    pub sport: String,
    pub role: String,
    pub coach_name: String,
    pub public_professional_email: String,
    pub ad_name: String,
    pub ad_email: String,
    pub source_urls: Vec<String>,
    pub last_observed: String,
}

impl FragmentRow {
    /// The four cells a citation has to contain, in match order.
    pub fn values(&self) -> [&str; 4] {
        [
            self.coach_name.as_str(),
            self.public_professional_email.as_str(),
            self.ad_name.as_str(),
            self.ad_email.as_str(),
        ]
    }

    /// The identity the reconciler keys on: school, state, sport, role and the person named.
    pub fn identity(&self) -> String {
        [
            &self.school,
            &self.state,
            &self.sport,
            &self.role,
            &self.coach_name,
            &self.ad_name,
        ]
        .iter()
        .map(|cell| normalize(cell))
        .collect::<Vec<_>>()
        .join("\u{1f}")
    }
}

/// Collapse a cell to its comparable form: whitespace runs to one space, trimmed, lowercased.
pub fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Flatten a page for matching: newlines, carriage returns and tabs become spaces and every
/// whitespace run collapses to one space, so a value broken across lines still matches literally.
pub fn flatten(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_space = false;
    for ch in raw.chars() {
        if ch.is_whitespace() {
            if !in_space {
                out.push(' ');
                in_space = true;
            }
        } else {
            out.push(ch);
            in_space = false;
        }
    }
    out
}

/// Whether any of the row's four value cells appears literally in the flattened page.
pub fn value_in(flat: &str, row: &FragmentRow) -> bool {
    row.values()
        .iter()
        .any(|value| !value.is_empty() && flat.contains(value))
}

/// Which role vocabulary the row's role label belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RoleClass {
    Director,
    Coach,
    /// A role with no vocabulary to corroborate (e.g. "Head XC Coach" on an AD row is a Coach).
    Other,
}

/// The role vocabulary a label selects, by the same substring rule the research lanes used.
fn role_class(role: &str) -> RoleClass {
    if role.contains("Director") {
        RoleClass::Director
    } else if role.contains("oach") {
        RoleClass::Coach
    } else {
        RoleClass::Other
    }
}

static ROLE_NEAR_DIRECTOR: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"(?i)(director|ADName|ADEmail|ADCell|ADPhone|"AD")"#).ok());
static ROLE_NEAR_COACH: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"(?i)(coach|CoachName|CoachEmail|"Coach")"#).ok());
static ROLE_CONTRA_DIRECTOR: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(coach|principal|superintendent|secretary|counselor|administrative|official representative|nurse|trainer)",
    )
    .ok()
});
static ROLE_CONTRA_COACH: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(athletic director|ADName|ADEmail|\bAD\b|principal|superintendent|secretary|counselor|administrative|official representative|nurse|trainer)",
    )
    .ok()
});

// Compiled once per process with no panic path: a pattern that cannot compile yields `None`, and a
// row whose class has no vocabulary keeps its class rather than being argued out of it.
fn near_pattern(class: RoleClass) -> Option<&'static Regex> {
    match class {
        RoleClass::Director => ROLE_NEAR_DIRECTOR.as_ref(),
        RoleClass::Coach => ROLE_NEAR_COACH.as_ref(),
        RoleClass::Other => None,
    }
}

fn contra_pattern(class: RoleClass) -> Option<&'static Regex> {
    match class {
        RoleClass::Director => ROLE_CONTRA_DIRECTOR.as_ref(),
        RoleClass::Coach => ROLE_CONTRA_COACH.as_ref(),
        RoleClass::Other => None,
    }
}

/// Snap an index to a character boundary so a byte window can never split a UTF-8 sequence.
fn boundary_at(text: &str, mut index: usize) -> usize {
    let len = text.len();
    if index > len {
        index = len;
    }
    while index < len && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}

/// The window `[offset - before, offset - before + len)` of the flattened page, boundary-aligned.
fn window_at(flat: &str, offset: usize, before: usize, len: usize) -> &str {
    let start = boundary_at(flat, offset.saturating_sub(before));
    let end = boundary_at(flat, start.saturating_add(len));
    flat.get(start..end).unwrap_or_default()
}

/// Whether the role is stated next to a matched value — the page's own words, in a 700-byte window
/// that opens 200 bytes before the value. A row whose role class has no vocabulary passes: there is
/// nothing to corroborate.
pub fn role_near(flat: &str, row: &FragmentRow) -> bool {
    let Some(regex) = near_pattern(role_class(&row.role)) else {
        return true;
    };
    row.values().iter().any(|value| {
        if value.is_empty() {
            return false;
        }
        match flat.find(value) {
            Some(offset) => {
                let window = window_at(flat, offset, ROLE_BEFORE, ROLE_WINDOW);
                regex.is_match(window)
            }
            None => false,
        }
    })
}

/// Whether the page states a *different* role for the matched person, in the tighter 300-byte
/// window that opens 100 bytes before the value. A title column sits there; an abbreviation legend
/// or a neighbouring staff entry does not.
pub fn role_contradicted(flat: &str, row: &FragmentRow) -> bool {
    let Some(regex) = contra_pattern(role_class(&row.role)) else {
        return false;
    };
    row.values().iter().any(|value| {
        if value.is_empty() {
            return false;
        }
        match flat.find(value) {
            Some(offset) => {
                let window = window_at(flat, offset, CONTRA_BEFORE, CONTRA_WINDOW);
                regex.is_match(window)
            }
            None => false,
        }
    })
}

/// The school-name form the NSAA export screen answers to: the trailing school word is dropped.
pub fn nsaa_school(school: &str) -> String {
    let trimmed = school.trim();
    for suffix in NSAA_SUFFIXES {
        let marker = format!(" {suffix}");
        if let Some(stripped) = trimmed.strip_suffix(marker.as_str()) {
            return stripped.trim_end().to_string();
        }
    }
    trimmed.to_string()
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
