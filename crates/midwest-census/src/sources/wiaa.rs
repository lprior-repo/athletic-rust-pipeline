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

use crate::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SourceIdentity,
    SourceNamespace, SourceRef, Sport,
};
use crate::net::{FetchOptions, FetchOutcome};
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{Context, Result};
use futures::stream::{self, StreamExt};
use std::collections::{BTreeMap, HashSet};

/// Host serving the WIAA school directory.
pub const HOST: &str = "https://schools.wiaawi.org";
/// Evidence/adapter slug used in [`SourceRef`]s.
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
    /// Restrict to these state codes when the provider spans several states.
    pub states: Vec<String>,
    /// School names to resolve when the provider has no bulk index.
    pub school_names: Vec<String>,
}

// -------------------------------------------------------------------------------------------------
// Parsed shapes
// -------------------------------------------------------------------------------------------------

/// One row of the per-letter school index (`#tblSchools`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IndexEntry {
    /// WIAA `OrganizationID` — the provider's stable school key.
    pub org_id: String,
    /// Full school name from the row's `title` attribute (`<h5>` is CSS-truncated for long names).
    pub name: String,
    /// `High School` / `Middle School`.
    pub level: String,
    /// City the school is listed under.
    pub city: String,
}

impl IndexEntry {
    /// The school page URL this row links to.
    pub fn page_url(&self) -> String {
        format!("{HOST}{SCHOOL_PATH}?orgID={}", self.org_id)
    }
}

/// One row of `#tblAdminList`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StaffRow {
    /// Published role label, e.g. `Athletic Director` or `AD Admin Assistant`.
    pub role: String,
    pub name: String,
    /// Decoded address, `None` when the cell carries no address.
    pub email: Option<String>,
}

/// One row of `#tblCoachList`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoachRow {
    /// Published sport label, e.g. `Boys Track and Field`.
    pub sport: String,
    pub name: String,
    /// Published role label; observed as `Head Coach` on every row of this surface.
    pub role: String,
    pub email: Option<String>,
}

/// The parsed content of one school page.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SchoolPage {
    /// `<label class="JumboMain">` — the school's own name.
    pub name: String,
    pub level: Option<String>,
    pub city: Option<String>,
    /// `Conference (Default)`. WIAA's conference field is what the canonical model calls
    /// `classification`.
    pub conference: Option<String>,
    pub enrollment: Option<u32>,
    pub website: Option<String>,
    pub admins: Vec<StaffRow>,
    pub coaches: Vec<CoachRow>,
}

/// Canonical entities for one school, plus a transcript of what the page offered and the model did
/// not take.
#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub coaches: Vec<CanonicalCoach>,
    /// Distinct non-director administration roles seen on the page (superintendent, principal,
    /// secretary, AD admin assistant, …). Never emitted as a coach or an athletic director.
    pub skipped_admin_roles: Vec<String>,
    /// Coach-table rows dropped because the sport is not TF/XC, the role is not a coaching role, or
    /// the row carries no person name.
    pub skipped_coach_rows: usize,
}

// -------------------------------------------------------------------------------------------------
// String primitives (no regex: nothing here can panic, every offset is bounds-checked)
// -------------------------------------------------------------------------------------------------

/// Byte offset of `needle` at or after `from`.
fn find_from(haystack: &str, needle: &str, from: usize) -> Option<usize> {
    haystack
        .get(from..)?
        .find(needle)
        .map(|offset| from + offset)
}

/// Text of the first `<tag …>…</tag>` at or after `from`, plus the offset just past it.
fn element_text(html: &str, tag: &str, from: usize) -> Option<(String, usize)> {
    let open_marker = format!("<{tag}");
    let close_marker = format!("</{tag}>");
    let open = find_from(html, &open_marker, from)?;
    let open_end = find_from(html, ">", open)?.checked_add(1)?;
    let close = find_from(html, &close_marker, open_end)?;
    let text = html
        .get(open_end..close)
        .map(strip_tags)
        .unwrap_or_default();
    let next = close.checked_add(close_marker.len())?;
    Some((text, next))
}

/// Bodies of every `<tag …>…</tag>` inside `html`, in document order. The cursor always advances to
/// just past a closing tag, so the loop terminates on any input.
fn element_bodies<'a>(html: &'a str, tag: &str) -> Vec<&'a str> {
    let open_marker = format!("<{tag}");
    let close_marker = format!("</{tag}>");
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while let Some(open) = find_from(html, &open_marker, cursor) {
        let Some(open_end) = find_from(html, ">", open).and_then(|at| at.checked_add(1)) else {
            break;
        };
        let Some(close) = find_from(html, &close_marker, open_end) else {
            break;
        };
        if let Some(body) = html.get(open_end..close) {
            out.push(body);
        }
        match close.checked_add(close_marker.len()) {
            Some(next) if next > cursor => cursor = next,
            _ => break,
        }
    }
    out
}

/// First `attribute="value"` inside `html`.
fn attribute_value(html: &str, attribute: &str) -> Option<String> {
    let marker = format!("{attribute}=\"");
    let start = find_from(html, &marker, 0)?.checked_add(marker.len())?;
    let rest = html.get(start..)?;
    let end = rest.find('"')?;
    rest.get(..end).map(|value| clean(&unescape(value)))
}

/// Text of every `<label class="<class>">…</label>` inside `html`, in document order.
fn labelled_texts(html: &str, class: &str) -> Vec<String> {
    let marker = format!("<label class=\"{class}\">");
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while let Some(at) = find_from(html, &marker, cursor) {
        let Some(start) = at.checked_add(marker.len()) else {
            break;
        };
        let Some(end) = find_from(html, "</label>", start) else {
            break;
        };
        if let Some(text) = html.get(start..end) {
            out.push(strip_tags(text));
        }
        match end.checked_add("</label>".len()) {
            Some(next) if next > cursor => cursor = next,
            _ => break,
        }
    }
    out
}

/// The `<table id="<table_id>">…</table>` slice, starting at the id attribute.
fn table_slice<'a>(html: &'a str, table_id: &str) -> Option<&'a str> {
    let marker = format!("id=\"{table_id}\"");
    let at = find_from(html, &marker, 0)?;
    let end = find_from(html, "</table>", at)?;
    html.get(at..end)
}

/// Cell `index` of a row, or `""` when the row is shorter than that.
fn nth<'a>(cells: &[&'a str], index: usize) -> &'a str {
    cells.get(index).copied().unwrap_or("")
}

/// Element `index` of an owned list, or `""` when the list is shorter than that.
fn nth_owned(values: &[String], index: usize) -> &str {
    values.get(index).map(String::as_str).unwrap_or("")
}

/// Remove markup, decode the entities this site emits, and collapse whitespace.
fn strip_tags(fragment: &str) -> String {
    let mut out = String::with_capacity(fragment.len());
    let mut depth = 0usize;
    for ch in fragment.chars() {
        match ch {
            '<' => depth = depth.saturating_add(1),
            '>' => depth = depth.saturating_sub(1),
            _ if depth == 0 => out.push(ch),
            _ => {}
        }
    }
    clean(&unescape(&out))
}

/// Decode the HTML entities observed in WIAA payloads.
fn unescape(value: &str) -> String {
    value
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&amp;", "&")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&times;", "×")
}

fn collapse_whitespace(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut in_space = false;
    for ch in value.trim().chars() {
        if ch.is_whitespace() {
            in_space = true;
        } else {
            if in_space && !out.is_empty() {
                out.push(' ');
            }
            in_space = false;
            out.push(ch);
        }
    }
    out
}

fn clean(value: &str) -> String {
    collapse_whitespace(value)
}

/// `None` for an empty or whitespace value, so optional fields stay absent rather than empty.
fn nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

/// Fetch options for one request. `options.refresh` and the run-level `ctx.refresh` both ask for a
/// cache bypass, so either flag wins.
fn fetch_options(ctx: &AdapterContext<'_>, options: &Options) -> FetchOptions {
    FetchOptions {
        refresh: options.refresh || ctx.refresh,
        ..ctx.fetch_options()
    }
}

/// A published value that is not a placeholder. WIAA writes `N/A` where a field does not apply
/// (e.g. the conference of a charter school), and a placeholder must not become a canonical field.
fn meaningful(value: &str) -> Option<String> {
    nonempty(value).filter(|value| {
        !matches!(
            value.to_ascii_lowercase().as_str(),
            "n/a" | "na" | "none" | "-" | "--" | "unknown" | "tbd"
        )
    })
}

fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

// -------------------------------------------------------------------------------------------------
// Email decoding
// -------------------------------------------------------------------------------------------------

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Decode Cloudflare's `data-cfemail` payload exactly as the page's own `email-decode.min.js` does:
/// the first byte is the XOR key, every following byte is one character of the address.
///
/// Returns `None` for malformed input and for a payload that does not decode to an address, so a
/// junk attribute can never become a `professional_email`.
pub fn decode_cfemail(encoded: &str) -> Option<String> {
    let hex = encoded.trim();
    let bytes = hex.as_bytes();
    if bytes.len() < 4 || !bytes.len().is_multiple_of(2) {
        return None;
    }
    let mut decoded_bytes = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let high = hex_nibble(*pair.first()?)?;
        let low = hex_nibble(*pair.get(1)?)?;
        decoded_bytes.push(high.checked_shl(4)?.checked_add(low)?);
    }
    let key = *decoded_bytes.first()?;
    let decoded: String = decoded_bytes
        .iter()
        .skip(1)
        .map(|byte| char::from(*byte ^ key))
        .collect();
    valid_email(&decoded)
}

/// A published address, or nothing: rejects blanks, placeholders and anything without a domain dot.
fn valid_email(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.contains(char::is_whitespace) {
        return None;
    }
    let mut parts = trimmed.split('@');
    let local = parts.next().unwrap_or("");
    let domain = parts.next().unwrap_or("");
    if parts.next().is_some() || local.is_empty() || !domain.contains('.') {
        return None;
    }
    if trimmed.contains(['[', ']']) {
        return None;
    }
    Some(trimmed.to_string())
}

/// Address from a table cell: the Cloudflare payload when present, otherwise a plain `mailto:` href.
fn cell_email(cell: &str) -> Option<String> {
    let marker = "data-cfemail=\"";
    if let Some(start) = find_from(cell, marker, 0).and_then(|at| at.checked_add(marker.len())) {
        if let Some(rest) = cell.get(start..) {
            if let Some(end) = rest.find('"') {
                if let Some(decoded) = rest.get(..end).and_then(decode_cfemail) {
                    return Some(decoded);
                }
            }
        }
    }
    let mailto = find_from(cell, "mailto:", 0).and_then(|at| at.checked_add("mailto:".len()))?;
    let rest = cell.get(mailto..)?;
    let end = rest.find(['"', '\'', '<', '?']).unwrap_or(rest.len());
    valid_email(rest.get(..end)?)
}

// -------------------------------------------------------------------------------------------------
// Page parsing
// -------------------------------------------------------------------------------------------------

/// WIAA `OrganizationID`s listed by one letter fragment, in page order.
///
/// Returns an empty vector for an empty or unrecognised payload — the `LetterBtn=-1` fragment is a
/// legitimate empty response, not an error.
pub fn parse_directory_letter(html: &str) -> Vec<IndexEntry> {
    let Some(table) = table_slice(html, "tblSchools") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for row in element_bodies(table, "tr") {
        let Some(org_id) = row_org_id(row) else {
            continue;
        };
        let name = attribute_value(row, "title")
            .and_then(|value| meaningful(&value))
            .or_else(|| {
                element_text(row, "h5", 0)
                    .map(|(text, _)| text)
                    .and_then(|text| meaningful(&text))
            })
            .unwrap_or_default();
        let labels = labelled_texts(row, "gridTextDataTables");
        out.push(IndexEntry {
            org_id,
            name,
            level: clean(nth_owned(&labels, 0)),
            city: clean(nth_owned(&labels, 2)),
        });
    }
    out
}

/// `orgID` from a row's `GetDirectorySchool` link.
fn row_org_id(row: &str) -> Option<String> {
    let marker = "GetDirectorySchool?orgID=";
    let start = find_from(row, marker, 0)?.checked_add(marker.len())?;
    let digits: String = row
        .get(start..)?
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

/// Value of the `<span>Label</span><h5 …>Value</h5>` pattern used by the school identity block.
fn labeled_value(html: &str, label: &str) -> Option<String> {
    let marker = format!("<span>{label}</span>");
    let start = find_from(html, &marker, 0)?.checked_add(marker.len())?;
    let (value, _) = element_text(html, "h5", start)?;
    meaningful(&value)
}

/// Text of the first non-empty `<label class="<class>">…</label>`.
fn label_text(html: &str, class: &str) -> Option<String> {
    labelled_texts(html, class)
        .into_iter()
        .find_map(|value| meaningful(&value))
}

/// `Enrollment (<school year>)` → the `School:` total.
pub fn parse_enrollment(html: &str) -> Option<u32> {
    let label = "<span>School:</span>";
    let at = find_from(html, "<span>Enrollment (", 0)?;
    let after = find_from(html, label, at)?.checked_add(label.len())?;
    let (value, _) = element_text(html, "b", after)?;
    value.trim().parse::<u32>().ok()
}

/// href of the anchor ending in `marker` (the school page's `Website` button). Anything that is not
/// an absolute http(s) URL is ignored rather than stored as a website.
fn anchor_href_before(html: &str, marker: &str) -> Option<String> {
    let at = find_from(html, marker, 0)?;
    let before = html.get(..at)?;
    let start = before.rfind("href=\"")?.checked_add("href=\"".len())?;
    let rest = html.get(start..)?;
    let end = rest.find('"')?;
    let href = rest.get(..end)?.trim();
    if href.starts_with("https://") || href.starts_with("http://") {
        nonempty(href)
    } else {
        None
    }
}

/// Name of the person in a table cell: the `<b>` element when present, otherwise the whole cell.
fn cell_person(cell: &str) -> String {
    element_text(cell, "b", 0)
        .map(|(text, _)| text)
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| strip_tags(cell))
}

/// Parse one `GetDirectorySchool` page. A payload that is not a directory page (empty body, JSON
/// error, truncated HTML) yields a default page rather than an error, so it can neither panic nor
/// mint a school.
pub fn parse_school_page(html: &str) -> SchoolPage {
    let mut admins = Vec::new();
    if let Some(table) = table_slice(html, "tblAdminList") {
        for row in element_bodies(table, "tr") {
            let cells = element_bodies(row, "td");
            let role = strip_tags(nth(&cells, 1));
            let name = cell_person(nth(&cells, 2));
            if role.is_empty() || name.is_empty() {
                continue;
            }
            admins.push(StaffRow {
                role,
                name,
                email: cell_email(nth(&cells, 3)),
            });
        }
    }

    let mut coaches = Vec::new();
    if let Some(table) = table_slice(html, "tblCoachList") {
        for row in element_bodies(table, "tr") {
            let cells = element_bodies(row, "td");
            let sport = strip_tags(nth(&cells, 1));
            let name = cell_person(nth(&cells, 2));
            let role = strip_tags(nth(&cells, 3));
            if sport.is_empty() || name.is_empty() {
                continue;
            }
            coaches.push(CoachRow {
                sport,
                name,
                role,
                email: cell_email(nth(&cells, 4)),
            });
        }
    }

    SchoolPage {
        name: label_text(html, "JumboMain").unwrap_or_default(),
        level: labeled_value(html, "Level"),
        city: labeled_value(html, "City"),
        conference: labeled_value(html, "Conference (Default)"),
        enrollment: parse_enrollment(html),
        website: anchor_href_before(html, "&nbsp;Website</a>"),
        admins,
        coaches,
    }
}

// -------------------------------------------------------------------------------------------------
// Label mapping
// -------------------------------------------------------------------------------------------------

/// Map a published sport label onto the sport ontology plus the gender side it covers.
///
/// `Boys Track and Field` → outdoor track, boys; `Girls Cross Country` → cross country, girls;
/// `Coed …` → mixed. Any other sport returns `None`, which is what keeps this adapter to TF/XC.
pub fn parse_sport_label(label: &str) -> Option<(Sport, Gender)> {
    let lowered = label.trim().to_ascii_lowercase();
    let sport = if lowered.contains("cross country") || lowered.contains("cross-country") {
        Sport::CrossCountry
    } else if lowered.contains("indoor") {
        Sport::IndoorTrack
    } else if lowered.contains("track") {
        // WIAA sanctions no indoor season: every track row on this surface is outdoor.
        Sport::OutdoorTrack
    } else {
        return None;
    };
    let gender = if lowered.contains("girls") || lowered.contains("women") {
        Gender::Girls
    } else if lowered.contains("boys") || lowered.contains("men") {
        Gender::Boys
    } else if lowered.contains("coed") || lowered.contains("co-ed") {
        Gender::Mixed
    } else {
        Gender::Unknown
    };
    Some((sport, gender))
}

/// Map a `#tblCoachList` role label onto the coach-role vocabulary.
///
/// Only published coaching roles are accepted; anything without the word "coach" is skipped.
pub fn parse_coach_role(label: &str) -> Option<CoachRole> {
    let lowered = label.trim().to_ascii_lowercase();
    if !lowered.contains("coach") {
        return None;
    }
    if lowered.contains("assistant") || lowered.contains("asst") {
        return Some(CoachRole::AssistantCoach);
    }
    if lowered.contains("head") {
        return Some(CoachRole::HeadCoach);
    }
    Some(CoachRole::Unknown)
}

/// Roles that mention an athletic director without being the school's athletic director: WIAA
/// publishes office staff ("AD Admin Assistant") in the same administration table as the AD, and its
/// assistant/associate AD rows are a different person from the director.
const NON_DIRECTOR_TOKENS: [&str; 10] = [
    "secretary",
    "administrative assistant",
    "admin assistant",
    "assistant",
    "asst",
    "associate",
    "trainer",
    "principal",
    "superintendent",
    "business manager",
];

/// Map a `#tblAdminList` role label onto a canonical role.
///
/// `Some(AthleticDirector)` only for the athletic/activities director. Office, medical, building and
/// assistant-administration roles return `None`, so they can never be imported as an AD or a coach.
pub fn parse_admin_role(label: &str) -> Option<CoachRole> {
    let lowered = label.trim().to_ascii_lowercase();
    if !(lowered.contains("athletic director") || lowered.contains("activities director")) {
        return None;
    }
    if NON_DIRECTOR_TOKENS
        .iter()
        .any(|token| lowered.contains(token))
    {
        return None;
    }
    Some(CoachRole::AthleticDirector)
}

/// Strip leading honorifics so "Coach Smith" and "Smith" mint the same coach identity.
pub fn strip_honorific(value: &str) -> String {
    let mut parts: Vec<&str> = value.split_whitespace().collect();
    while let Some(first) = parts.first() {
        let token = first.trim_end_matches('.').to_ascii_lowercase();
        if matches!(
            token.as_str(),
            "mr" | "mrs" | "ms" | "miss" | "dr" | "coach" | "sir" | "rev"
        ) {
            parts.remove(0);
        } else {
            break;
        }
    }
    if parts.is_empty() {
        clean(value)
    } else {
        parts.join(" ")
    }
}

// -------------------------------------------------------------------------------------------------
// Entity mapping
// -------------------------------------------------------------------------------------------------

/// Build the canonical school and its AD/head-coach rows for one directory page.
///
/// The school name comes from the page (`JumboMain`), falling back to the index row's `title`
/// attribute when the page omits it. Returns `None` when neither carries a name: nothing is minted
/// from an empty document.
pub fn school_entities(
    entry: &IndexEntry,
    page: &SchoolPage,
    observed_on: &str,
) -> Option<SchoolExtract> {
    let name = meaningful(&page.name).or_else(|| meaningful(&entry.name))?;
    let page_url = entry.page_url();
    let source = SourceRef::new(SOURCE_ID, Some(page_url.clone()));
    let namespace = SourceNamespace::AssociationSchool {
        association: ASSOCIATION.to_string(),
    };

    let (mut school, school_id) = CanonicalSchool::new("WI", &name, normalize_name(&name));
    school.city = page
        .city
        .as_deref()
        .and_then(meaningful)
        .or_else(|| meaningful(&entry.city));
    if let Some(city) = school.city.as_ref() {
        school.aliases.push(format!("{city} WI"));
    }
    school.association = Some(ASSOCIATION.to_string());
    school.classification = page.conference.as_deref().and_then(meaningful);
    school.enrollment = page.enrollment;
    school.school_website = page.website.as_deref().and_then(meaningful);
    school.source_identities.push(
        SourceIdentity::new(namespace.clone(), entry.org_id.clone()).with_url(page_url.clone()),
    );
    school
        .evidence
        .push(Evidence::parsed(source.clone(), observed_on));

    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut skipped_admin_roles: Vec<String> = Vec::new();

    for admin in &page.admins {
        let Some(role) = parse_admin_role(&admin.role) else {
            if !skipped_admin_roles.iter().any(|seen| seen == &admin.role) {
                skipped_admin_roles.push(admin.role.clone());
            }
            continue;
        };
        let person = strip_honorific(&admin.name);
        if person.is_empty() {
            continue;
        }
        let mut coach = CanonicalCoach::new(&school_id, person, None, Gender::Mixed, role);
        coach.professional_email = admin.email.as_deref().and_then(valid_email);
        coach
            .evidence
            .push(Evidence::parsed(source.clone(), observed_on));
        if seen.insert(coach.id.to_string()) {
            coaches.push(coach);
        }
    }

    let mut skipped_coach_rows = 0usize;
    for row in &page.coaches {
        let Some((sport, gender)) = parse_sport_label(&row.sport) else {
            skipped_coach_rows = skipped_coach_rows.saturating_add(1);
            continue;
        };
        let Some(role) = parse_coach_role(&row.role) else {
            skipped_coach_rows = skipped_coach_rows.saturating_add(1);
            continue;
        };
        let person = strip_honorific(&row.name);
        if person.is_empty() {
            skipped_coach_rows = skipped_coach_rows.saturating_add(1);
            continue;
        }
        let mut coach = CanonicalCoach::new(&school_id, person, Some(sport), gender, role);
        coach.professional_email = row.email.as_deref().and_then(valid_email);
        coach
            .evidence
            .push(Evidence::parsed(source.clone(), observed_on));
        if seen.insert(coach.id.to_string()) {
            coaches.push(coach);
        }
    }

    Some(SchoolExtract {
        school,
        coaches,
        skipped_admin_roles,
        skipped_coach_rows,
    })
}

// -------------------------------------------------------------------------------------------------
// Collection
// -------------------------------------------------------------------------------------------------

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

/// Walk the WIAA directory and emit canonical schools plus AD/head-coach rows.
///
/// Resumable: a school page is fetched only when `WI:<orgID>` is absent from the `wiaa_schools`
/// journal, and both journals carry the same `WI:<orgID>` key.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let mut report = AdapterReport::new("wiaa", "schools");
    let observed_on = if options.observed_on.trim().is_empty() {
        ctx.observed_on.clone()
    } else {
        options.observed_on.clone()
    };
    let before = ctx.fetcher.stats().await;

    if !options.states.is_empty()
        && !options
            .states
            .iter()
            .any(|state| state.eq_ignore_ascii_case("WI"))
    {
        let after = ctx.fetcher.stats().await;
        report.requests = after.requests.saturating_sub(before.requests);
        report.note(format!(
            "states {:?} do not include WI; this adapter covers Wisconsin only",
            options.states
        ));
        return Ok(report);
    }

    // -- index: bounded-concurrency fetch per directory letter (N=8) ----------------------------
    const LETTER_CONCURRENCY: usize = 8;
    let letters = letters_for(&options.school_names);
    // Collect (letter_index, result) pairs so we can process in submission order.
    let letter_results: Vec<(usize, Result<FetchOutcome>)> =
        stream::iter(letters.iter().enumerate())
            .map(|(i, letter)| {
                let url = format!("{HOST}{INDEX_PATH}?LetterBtn={letter}");
                let opts = fetch_options(ctx, options);
                async move { (i, ctx.fetcher.get(&url, &opts).await) }
            })
            .buffer_unordered(LETTER_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;

    // Process results in submission order for deterministic error tracking.
    let mut index: Vec<IndexEntry> = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();
    let mut letters_ok = 0usize;
    let mut first_problem: Option<String> = None;
    for (i, result) in letter_results {
        let letter = letters[i];
        match result {
            Ok(outcome) if outcome.status == 200 => {
                letters_ok = letters_ok.saturating_add(1);
                for entry in parse_directory_letter(&outcome.text()) {
                    if seen_ids.insert(entry.org_id.clone()) {
                        index.push(entry);
                    }
                }
            }
            Ok(outcome) => {
                report.errors = report.errors.saturating_add(1);
                let problem = format!("letter {letter} returned HTTP {}", outcome.status);
                report.note(problem.clone());
                if first_problem.is_none() {
                    first_problem = Some(problem);
                }
            }
            Err(error) => {
                report.errors = report.errors.saturating_add(1);
                let problem = format!("letter {letter}: {error}");
                report.note(problem.clone());
                if first_problem.is_none() {
                    first_problem = Some(problem);
                }
            }
        }
    }
    if letters_ok == 0 {
        let after = ctx.fetcher.stats().await;
        report.requests = after.requests.saturating_sub(before.requests);
        report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
        anyhow::bail!(
            "WIAA directory index {HOST}{INDEX_PATH} returned no usable letter fragment ({} request(s) attempted): {}",
            letters.len(),
            first_problem.unwrap_or_else(|| "no response".to_string())
        );
    }

    let mut levels: BTreeMap<String, u64> = BTreeMap::new();
    for entry in &index {
        let level = meaningful(&entry.level).unwrap_or_else(|| "unstated".to_string());
        let slot = levels.entry(level).or_insert(0);
        *slot = slot.saturating_add(1);
    }
    let level_summary = levels
        .iter()
        .map(|(level, count)| format!("{level}={count}"))
        .collect::<Vec<_>>()
        .join(", ");

    // -- schools: one request per school ---------------------------------------------------------
    let done = ctx.store.journal_keys("wiaa_schools")?;
    let wanted: Option<HashSet<String>> = if options.school_names.is_empty() {
        None
    } else {
        Some(
            options
                .school_names
                .iter()
                .map(|name| normalize_name(name))
                .collect(),
        )
    };

    let mut processed = 0usize;
    let mut skipped_done = 0usize;
    let mut skipped_filter = 0usize;
    let mut not_found = 0usize;
    let mut page_failures = 0usize;
    let mut coach_rows = 0usize;
    let mut with_email = 0u64;
    let mut skipped_admin_roles: Vec<String> = Vec::new();
    let mut skipped_coach_rows = 0usize;

    // -- schools: bounded-concurrency fetch per school page (N=8) ----------------------------
    const SCHOOL_CONCURRENCY: usize = 8;
    // Phase 1: identify eligible schools (skip already journaled, apply filter).
    let eligible: Vec<(usize, &IndexEntry)> = index
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            if let Some(wanted) = wanted.as_ref() {
                if !wanted.contains(&normalize_name(&entry.name)) {
                    skipped_filter = skipped_filter.saturating_add(1);
                    return false;
                }
            }
            let key = format!("WI:{}", entry.org_id);
            if done.contains(&key) {
                skipped_done = skipped_done.saturating_add(1);
                return false;
            }
            true
        })
        .collect();

    // Phase 2: fetch all school pages in parallel (bounded concurrency N=8).
    // Each fetch is independent — same host, but rate-limited by the fetcher's gate.
    let fetch_results: Vec<(usize, Result<FetchOutcome>)> = stream::iter(eligible)
        .map(|(idx, entry)| {
            let url = entry.page_url();
            let page_options = FetchOptions {
                allow_not_found: true,
                ..fetch_options(ctx, options)
            };
            async move { (idx, ctx.fetcher.get(&url, &page_options).await) }
        })
        .buffer_unordered(SCHOOL_CONCURRENCY)
        .collect::<Vec<_>>()
        .await;

    // Phase 3: process results in submission order (deterministic).
    for (idx, result) in fetch_results {
        let entry = &index[idx];
        let key = format!("WI:{}", entry.org_id);

        // Respect limit after collection.
        if let Some(limit) = options.limit {
            if processed >= limit {
                break;
            }
        }

        let outcome = match result {
            Ok(outcome) => outcome,
            Err(error) => {
                page_failures = page_failures.saturating_add(1);
                report.errors = report.errors.saturating_add(1);
                if page_failures <= 5 {
                    report.note(format!("school {key}: {error}"));
                }
                continue;
            }
        };
        if outcome.status == 404 {
            not_found = not_found.saturating_add(1);
            continue;
        }
        if outcome.status != 200 {
            page_failures = page_failures.saturating_add(1);
            report.errors = report.errors.saturating_add(1);
            if page_failures <= 5 {
                report.note(format!("school {key}: HTTP {}", outcome.status));
            }
            continue;
        }

        let page = parse_school_page(&outcome.text());
        let Some(extract) = school_entities(entry, &page, &observed_on) else {
            page_failures = page_failures.saturating_add(1);
            report.errors = report.errors.saturating_add(1);
            if page_failures <= 5 {
                report.note(format!("school {key}: page carried no school name"));
            }
            continue;
        };

        ctx.store
            .append(Table::Schools, &extract.school)
            .with_context(|| format!("writing WIAA school {key}"))?;
        ctx.store
            .append_many(Table::Coaches, &extract.coaches)
            .with_context(|| format!("writing WIAA coaches for {key}"))?;

        let school_with_email = extract
            .coaches
            .iter()
            .filter(|coach| coach.professional_email.is_some())
            .count();
        with_email = with_email.saturating_add(count(school_with_email));
        coach_rows = coach_rows.saturating_add(extract.coaches.len());
        skipped_coach_rows = skipped_coach_rows.saturating_add(extract.skipped_coach_rows);
        for role in extract.skipped_admin_roles {
            if !skipped_admin_roles.iter().any(|seen| seen == &role) {
                skipped_admin_roles.push(role);
            }
        }

        let payload = serde_json::json!({
            "org_id": entry.org_id,
            "name": extract.school.name,
            "city": extract.school.city,
            "conference": extract.school.classification,
            "level": page.level,
            "coaches": extract.coaches.len(),
            "coaches_with_email": school_with_email,
        });
        ctx.store
            .journal_done("wiaa_schools", &key, &payload)
            .with_context(|| format!("journaling WIAA school {key}"))?;
        ctx.store
            .journal_done("wiaa_coaches", &key, &payload)
            .with_context(|| format!("journaling WIAA coaches for {key}"))?;

        processed = processed.saturating_add(1);
    }

    let after = ctx.fetcher.stats().await;
    report.rows = count(processed);
    report.requests = after.requests.saturating_sub(before.requests);
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    report.with_email = with_email;

    report.note(format!(
        "index: {} letter request(s), {} schools listed ({level_summary})",
        letters.len(),
        index.len()
    ));
    report.note(format!(
        "wrote {processed} schools and {coach_rows} AD/head-coach rows to the store"
    ));
    report.note(if coach_rows == 0 {
        "no coach or athletic-director row was published on the fetched pages".to_string()
    } else if with_email == 0 {
        "no email published on this surface".to_string()
    } else {
        let percent = with_email
            .checked_mul(100)
            .and_then(|scaled| scaled.checked_div(count(coach_rows)))
            .unwrap_or(0);
        format!("published coach/AD email fill rate: {with_email}/{coach_rows} rows ({percent}%)")
    });
    report.note(format!(
        "skipped {skipped_done} already journaled, {skipped_filter} outside the requested school names, {not_found} HTTP 404, {page_failures} page failures"
    ));
    report.note(format!(
        "not imported: {skipped_coach_rows} coach-table rows outside TF/XC or without a coaching role, {} non-director administration roles",
        count(skipped_admin_roles.len())
    ));
    if !skipped_admin_roles.is_empty() {
        let mut sorted = skipped_admin_roles;
        sorted.sort_unstable();
        report.note(format!(
            "non-director admin roles seen: {}",
            sorted.join(", ")
        ));
    }
    if let Some(limit) = options.limit {
        report.note(format!("limit applied: {limit} school(s)"));
    }
    Ok(report)
}

// -------------------------------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Verbatim byte range of the 2026-09-19 capture `wi-list-A.html`
    /// (`GET https://schools.wiaawi.org/Directory/School/DirectoryLetter?LetterBtn=A`, HTTP 200,
    /// 127,772 bytes): the "Showing 39 schools…" banner plus the `#tblSchools` header and its first
    /// six data rows, unmodified.
    const INDEX_A: &str = include_str!("../../tests/fixtures/wiaa/directory_letter_a.html");

    /// Verbatim byte range 109,601-148,863 of the capture `wi-school-1.html`
    /// (`GET https://schools.wiaawi.org/Directory/School/GetDirectorySchool?orgID=1`, HTTP 200,
    /// 179,419 bytes): the jumbotron, the identity block, the website buttons, `#tblAdminList` and
    /// `#tblCoachList`.
    const SCHOOL_ABBOTSFORD: &str =
        include_str!("../../tests/fixtures/wiaa/school_org1_abbotsford.html");

    /// Verbatim byte range 109,601-163,662 of the capture `wi-school-135.html` (`GET …?orgID=135`,
    /// HTTP 200, 200,357 bytes). Carries an `AD Admin Assistant` row that must never be imported.
    const SCHOOL_GET: &str =
        include_str!("../../tests/fixtures/wiaa/school_org135_gale_ettrick_trempealeau.html");

    /// Verbatim byte range 109,601-128,994 of the capture `wi-school-5151.html` (`GET …?orgID=5151`,
    /// HTTP 200, 149,086 bytes). Two directors and zero coach rows.
    const SCHOOL_SAILS: &str =
        include_str!("../../tests/fixtures/wiaa/school_org5151_sails_charter.html");

    const OBSERVED_ON: &str = "2026-09-20";

    fn entry_for(org_id: &str) -> IndexEntry {
        parse_directory_letter(INDEX_A)
            .into_iter()
            .find(|entry| entry.org_id == org_id)
            .unwrap_or_else(|| IndexEntry {
                org_id: org_id.to_string(),
                ..IndexEntry::default()
            })
    }

    fn extract_for(org_id: &str, fixture: &str) -> SchoolExtract {
        let entry = entry_for(org_id);
        let page = parse_school_page(fixture);
        school_entities(&entry, &page, OBSERVED_ON).expect("fixture page yields entities")
    }

    #[test]
    fn index_parsing_yields_org_ids_level_and_city() {
        let entries = parse_directory_letter(INDEX_A);
        assert_eq!(entries.len(), 6, "fixture keeps six real index rows");
        assert_eq!(entries[0].org_id, "1");
        assert_eq!(entries[0].name, "ABBOTSFORD");
        assert_eq!(entries[0].level, "High School");
        assert_eq!(entries[0].city, "Abbotsford");
        assert_eq!(
            entries[0].page_url(),
            format!("{HOST}{SCHOOL_PATH}?orgID=1")
        );
        // The `title` attribute is the untruncated name; the visible <h5> is CSS-truncated.
        let last = entries.last().expect("six entries");
        assert_eq!(last.name, "ADVANCED LEARNING ACADEMY OF WISCONSIN CHARTER");
        assert_eq!(last.level, "High School");
        assert!(entries.iter().any(|entry| entry.level == "Middle School"));
        let mut ids: Vec<&str> = entries.iter().map(|entry| entry.org_id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), entries.len(), "orgIDs are unique");
        assert!(entries.iter().all(|entry| !entry.org_id.is_empty()));
    }

    #[test]
    fn empty_index_fragment_yields_no_rows() {
        assert!(parse_directory_letter("").is_empty());
        assert!(parse_directory_letter("null").is_empty());
        // Shape of the `LetterBtn=-1` fragment: HTTP 200, 3,077 bytes, no school table.
        assert!(parse_directory_letter("<div class=\"alert\"></div>").is_empty());
    }

    #[test]
    fn school_page_yields_name_city_conference_ad_and_identity() {
        let page = parse_school_page(SCHOOL_ABBOTSFORD);
        assert_eq!(page.name, "Abbotsford");
        assert_eq!(page.city.as_deref(), Some("Abbotsford"));
        assert_eq!(page.conference.as_deref(), Some("Marawood"));
        assert_eq!(page.level.as_deref(), Some("High School"));
        assert_eq!(page.enrollment, Some(214));
        assert_eq!(
            page.website.as_deref(),
            Some("http://www.abbotsford.k12.wi.us")
        );

        let extract = extract_for("1", SCHOOL_ABBOTSFORD);
        assert_eq!(extract.school.name, "Abbotsford");
        assert_eq!(extract.school.state.as_deref(), Some("WI"));
        assert_eq!(extract.school.city.as_deref(), Some("Abbotsford"));
        assert_eq!(extract.school.association.as_deref(), Some("wiaa"));
        assert_eq!(extract.school.classification.as_deref(), Some("Marawood"));
        assert_eq!(extract.school.enrollment, Some(214));
        assert_eq!(
            extract.school.school_website.as_deref(),
            Some("http://www.abbotsford.k12.wi.us")
        );
        // Canonical id is deterministic from state + normalized name, so another provider that saw
        // Abbotsford mints the same school.
        assert_eq!(
            extract.school.id,
            CanonicalSchool::new("WI", "Abbotsford", normalize_name("Abbotsford")).1
        );
        let identity = extract
            .school
            .source_identities
            .first()
            .expect("one provider identity");
        assert_eq!(
            identity.namespace,
            SourceNamespace::AssociationSchool {
                association: "wiaa".to_string()
            }
        );
        assert_eq!(identity.id, "1");
        assert_eq!(
            identity.url.as_deref(),
            Some("https://schools.wiaawi.org/Directory/School/GetDirectorySchool?orgID=1")
        );
        assert_eq!(extract.school.evidence.len(), 1);
        assert_eq!(extract.school.evidence[0].observed_on, OBSERVED_ON);
        assert_eq!(extract.school.evidence[0].source.id, SOURCE_ID);
        assert!(extract.school.evidence[0]
            .source
            .url
            .as_deref()
            .is_some_and(|url| url.contains("orgID=1")));

        // Role and sport labels are scraped out of `<label>` elements; markup must never survive
        // into the parsed label (the role string feeds the director/coach classifier).
        assert!(!page.admins.is_empty(), "Abbotsford publishes office rows");
        for admin in &page.admins {
            assert!(
                !admin.role.contains('<') && !admin.role.contains('>'),
                "markup in admin role {:?}",
                admin.role
            );
        }
        for coach in &page.coaches {
            assert!(
                !coach.sport.contains('<') && !coach.role.contains('<'),
                "markup in coach row {:?} {:?}",
                coach.sport,
                coach.role
            );
        }

        let ad = extract
            .coaches
            .iter()
            .find(|coach| coach.role == CoachRole::AthleticDirector)
            .expect("Abbotsford publishes an athletic director");
        assert_eq!(ad.name, "Alex Larson");
        assert_eq!(ad.sport, None, "an AD is school-wide, never sport-bound");
        assert_eq!(ad.gender, Gender::Mixed);
        assert_eq!(
            ad.professional_email.as_deref(),
            Some("alarson@abbotsford.k12.wi.us")
        );
        assert_eq!(ad.evidence.len(), 1);
        assert_eq!(ad.evidence[0].observed_on, OBSERVED_ON);
    }

    #[test]
    fn coach_rows_map_to_sport_and_gender() {
        let extract = extract_for("1", SCHOOL_ABBOTSFORD);
        let find = |name: &str| -> Vec<&CanonicalCoach> {
            extract
                .coaches
                .iter()
                .filter(|coach| coach.name == name)
                .collect()
        };

        let knapmiller = find("JACOB KNAPMILLER");
        assert_eq!(knapmiller.len(), 2, "same person, two sport-gender rows");
        let mut pairs: Vec<(Sport, Gender)> = knapmiller
            .iter()
            .map(|coach| (coach.sport.expect("sport-bound"), coach.gender))
            .collect();
        pairs.sort_unstable();
        assert_eq!(
            pairs,
            vec![
                (Sport::OutdoorTrack, Gender::Boys),
                (Sport::OutdoorTrack, Gender::Girls)
            ]
        );
        assert!(knapmiller
            .iter()
            .all(|coach| coach.role == CoachRole::HeadCoach));
        assert!(knapmiller.iter().all(|coach| {
            coach.professional_email.as_deref() == Some("jknapmiller@abbotsford.k12.wi.us")
        }));

        let novak = find("Dillon Novak");
        assert_eq!(novak.len(), 1);
        assert_eq!(novak[0].sport, Some(Sport::CrossCountry));
        assert_eq!(novak[0].gender, Gender::Girls);
        assert_eq!(novak[0].role, CoachRole::HeadCoach);
        assert_eq!(
            novak[0].professional_email.as_deref(),
            Some("dnovak@abbotsford.k12.wi.us")
        );

        // Only TF/XC rows survive: Abbotsford publishes 14 coach rows, of which 3 are TF/XC.
        assert_eq!(
            extract.coaches.len(),
            4,
            "one AD + three TF/XC head coaches"
        );
        assert_eq!(extract.skipped_coach_rows, 11);
        for coach in &extract.coaches {
            if coach.role == CoachRole::AthleticDirector {
                continue;
            }
            assert!(matches!(
                coach.sport,
                Some(Sport::OutdoorTrack | Sport::CrossCountry)
            ));
        }

        // A second school: G-E-T's Paula Gold holds three TF/XC roles, all three must survive.
        let gets = extract_for("135", SCHOOL_GET);
        let gold: Vec<&CanonicalCoach> = gets
            .coaches
            .iter()
            .filter(|coach| coach.name == "Paula Gold")
            .collect();
        let mut gold_pairs: Vec<(String, Gender)> = gold
            .iter()
            .map(|coach| {
                let sport = coach.sport.expect("sport-bound");
                (format!("{sport:?}"), coach.gender)
            })
            .collect();
        gold_pairs.sort();
        assert_eq!(
            gold_pairs,
            vec![
                ("CrossCountry".to_string(), Gender::Boys),
                ("CrossCountry".to_string(), Gender::Girls),
                ("OutdoorTrack".to_string(), Gender::Girls),
            ]
        );
        assert_eq!(gets.coaches.len(), 5, "one AD + four TF/XC head coaches");
        assert_eq!(gets.skipped_coach_rows, 19);
    }

    #[test]
    fn sport_and_role_labels_are_mapped_strictly() {
        assert_eq!(
            parse_sport_label("Boys Track and Field"),
            Some((Sport::OutdoorTrack, Gender::Boys))
        );
        assert_eq!(
            parse_sport_label("Girls Track and Field"),
            Some((Sport::OutdoorTrack, Gender::Girls))
        );
        assert_eq!(
            parse_sport_label("Boys Cross Country"),
            Some((Sport::CrossCountry, Gender::Boys))
        );
        assert_eq!(
            parse_sport_label("Girls Cross Country"),
            Some((Sport::CrossCountry, Gender::Girls))
        );
        assert_eq!(
            parse_sport_label("Coed Track and Field"),
            Some((Sport::OutdoorTrack, Gender::Mixed))
        );
        assert_eq!(parse_sport_label("Boys Wrestling"), None);
        assert_eq!(parse_sport_label("Girls Swimming & Diving"), None);
        assert_eq!(parse_sport_label(""), None);

        assert_eq!(parse_coach_role("Head Coach"), Some(CoachRole::HeadCoach));
        assert_eq!(
            parse_coach_role("Assistant Coach"),
            Some(CoachRole::AssistantCoach)
        );
        assert_eq!(parse_coach_role("Volunteer"), None);
        assert_eq!(parse_coach_role(""), None);

        assert_eq!(
            parse_admin_role("Athletic Director"),
            Some(CoachRole::AthleticDirector)
        );
        // Real label from the captured sample (orgID 219 Madison East): a district-level director
        // published inside one school's administration table. It is a role-published director for
        // that school, so it is kept; the assistant AD row beside it is not.
        assert_eq!(
            parse_admin_role("City-Wide Athletic Director"),
            Some(CoachRole::AthleticDirector)
        );
        assert_eq!(
            parse_admin_role("Activities Director"),
            Some(CoachRole::AthleticDirector)
        );
        for office in [
            "AD Admin Assistant",
            "Assistant Athletic Director",
            "Athletic Director Secretary",
            "Athletic Trainer",
            "Principal",
            "Superintendent",
            "Business Manager",
            "",
        ] {
            assert_eq!(parse_admin_role(office), None, "office role {office:?}");
        }
    }

    #[test]
    fn office_staff_are_never_imported_as_coaches_or_directors() {
        let gets = extract_for("135", SCHOOL_GET);
        let names: Vec<&str> = gets
            .coaches
            .iter()
            .map(|coach| coach.name.as_str())
            .collect();
        // Real rows of the captured page: an office assistant inside the AD's table, plus the
        // building administration.
        for dropped in ["Sheryl Byom", "Michele Butler", "Jamie Oliver"] {
            assert!(
                !names.contains(&dropped),
                "non-director office row imported as a coach or AD: {dropped} in {names:?}"
            );
        }
        assert!(gets
            .skipped_admin_roles
            .iter()
            .any(|role| role == "AD Admin Assistant"));
        assert!(gets
            .skipped_admin_roles
            .iter()
            .any(|role| role == "Superintendent"));
        assert!(gets
            .skipped_admin_roles
            .iter()
            .any(|role| role == "Principal"));

        let directors: Vec<&CanonicalCoach> = gets
            .coaches
            .iter()
            .filter(|coach| coach.role == CoachRole::AthleticDirector)
            .collect();
        assert_eq!(directors.len(), 1, "exactly one AD row");
        assert_eq!(directors[0].name, "Jake Perner");
        assert_eq!(
            directors[0].professional_email.as_deref(),
            Some("jakeperner@getschools.k12.wi.us")
        );
    }

    #[test]
    fn school_without_coach_rows_yields_no_coaching_rows() {
        let extract = extract_for("5151", SCHOOL_SAILS);
        assert_eq!(extract.school.name, "S.A.I.L.S. CHARTER");
        assert_eq!(extract.school.city.as_deref(), Some("Sparta"));
        assert_eq!(extract.school.source_identities[0].id, "5151");
        // WIAA prints "N/A" for this school's conference; a placeholder must not become a field.
        assert_eq!(extract.school.classification, None);
        assert_eq!(extract.coaches.len(), 2, "two directors, zero coach rows");
        assert!(extract
            .coaches
            .iter()
            .all(|coach| coach.role == CoachRole::AthleticDirector));
        assert_eq!(extract.skipped_coach_rows, 0);
        let names: Vec<&str> = extract
            .coaches
            .iter()
            .map(|coach| coach.name.as_str())
            .collect();
        assert_eq!(names, vec!["John Blaha", "Adam Dow"]);
    }

    #[test]
    fn empty_or_malformed_payload_yields_zero_rows() {
        for payload in [
            "",
            "null",
            "   ",
            "<html></html>",
            "{\"error\":\"not found\"}",
            "<table id=\"tblSchools\">",
        ] {
            assert_eq!(
                parse_school_page(payload),
                SchoolPage::default(),
                "payload {payload:?}"
            );
            assert!(
                parse_directory_letter(payload).is_empty(),
                "payload {payload:?}"
            );
            assert_eq!(parse_enrollment(payload), None);
            assert!(decode_cfemail(payload).is_none());
        }
        assert!(decode_cfemail("").is_none());
        assert!(decode_cfemail("abc").is_none());
        assert!(decode_cfemail("zzzz").is_none());
        assert!(decode_cfemail("00010203").is_none());

        // An index row with an orgID but no name mints nothing.
        let nameless = IndexEntry {
            org_id: "9999".to_string(),
            ..IndexEntry::default()
        };
        assert!(school_entities(&nameless, &SchoolPage::default(), OBSERVED_ON).is_none());
    }

    #[test]
    fn cfemail_decoding_matches_published_addresses() {
        // Verbatim `data-cfemail` payloads from the Abbotsford captures.
        assert_eq!(
            decode_cfemail("7f1514111e0f121613131a0d3f1e1d1d100b0c19100d1b51144e4d510816510a0c")
                .as_deref(),
            Some("jknapmiller@abbotsford.k12.wi.us")
        );
        assert_eq!(
            decode_cfemail("f0919c9182839f9eb09192929f8483969f8294de9bc1c2de8799de8583").as_deref(),
            Some("alarson@abbotsford.k12.wi.us")
        );
    }

    #[test]
    fn honorifics_are_stripped_from_person_names() {
        assert_eq!(strip_honorific("Mr. Barry Mink"), "Barry Mink");
        assert_eq!(strip_honorific("Coach Dana Bell"), "Dana Bell");
        assert_eq!(strip_honorific("Dr. Ana Ruiz"), "Ana Ruiz");
        assert_eq!(strip_honorific("JACOB  KNAPMILLER"), "JACOB KNAPMILLER");
        assert_eq!(strip_honorific("   "), "");
    }

    /// Measured, not assumed: the fill rate below is what the captured pages actually publish.
    #[test]
    fn measured_email_fill_rate_on_captured_pages() {
        let mut rows = 0usize;
        let mut with_email = 0usize;
        for (org_id, fixture) in [
            ("1", SCHOOL_ABBOTSFORD),
            ("135", SCHOOL_GET),
            ("5151", SCHOOL_SAILS),
        ] {
            let extract = extract_for(org_id, fixture);
            rows = rows.saturating_add(extract.coaches.len());
            with_email = with_email.saturating_add(
                extract
                    .coaches
                    .iter()
                    .filter(|coach| coach.professional_email.is_some())
                    .count(),
            );
        }
        assert_eq!(
            (with_email, rows),
            (11, 11),
            "captured pages publish an address for every emitted AD/coach row"
        );
        assert!(
            with_email > 0,
            "the WIAA directory does publish coach addresses"
        );
    }
}
