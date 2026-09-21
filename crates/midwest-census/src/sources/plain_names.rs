//! NDHSAA + NSAA adapter (North Dakota, Nebraska): school universes and coach **names**.
//!
//! Two name-only state associations share this file because their contract is identical: a canonical
//! school per member school plus coach/athletic-director rows with **no email layer at all**. The
//! value here is the school universe and coach-name attribution, not contact addresses, so every
//! entity written by this adapter has `professional_email == None` and the report says so.
//!
//! # Requests (no browser, no Athletic.net, both server-rendered)
//!
//! * **North Dakota** — `GET https://ndhsaa.com/schools` lists all 169 member schools, then one
//!   `GET https://ndhsaa.com/schools/<id>/<slug>` per school carries the staff block
//!   (Superintendent, Principal, Athletic/Activities Director …) and the
//!   `Sport/Activity Offering | Coaches` table.
//! * **Nebraska** — `GET https://secure.nsaahome.org/nsaaforms/direxportscreen.php` returns the
//!   request form plus the `<option>` list of all **312** member schools, then one
//!   `GET …?session=&school=<name>` per school returns its full record: Superintendent, Principal,
//!   Athletics/Activities Director(s), and one row per sport. NSAA publishes no numeric school id in
//!   any surface, so the published school name *is* the provider key space.
//!
//!   The screen also offers a bulk form (`session= `, `school=View all schools`,
//!   `submit=See School Info`) that renders all 312 schools into one 1,085,584-byte body — but it
//!   needs **49.4 s** to render (measured HTTP 200 2026-09-20T14:34:47Z), past the fetcher's 45 s
//!   client timeout, so that single response is never usable in this harness. A per-school request
//!   answers in ~0.3 s and journals per school, which is what the walk below does.
//!
//! # Role policy (hard rule)
//!
//! * Office/building staff are never emitted as coaches or athletic directors: the token list in
//!   [`OFFICE_ROLE_TOKENS`] drops secretaries, business managers, technology directors, trainers,
//!   principals and superintendents **before** any director/coach match is attempted, so an athletic
//!   director's secretary and a superintendent are both unpresent regardless of seniority.
//! * NDHSAA's table is headed "Coaches" and never says which name is the head coach, so sport-scoped
//!   North Dakota rows carry [`CoachRole::Unknown`] rather than an invented head/assistant split.
//!   Nebraska publishes exactly one coach per sport, so its sport rows are [`CoachRole::HeadCoach`].
//! * A multi-name cell ("Trey Schlueter/Katie Winters") is split into one entity per person — the
//!   alternative is storing two humans inside one `name` field.
//!
//! # Deliberately never read
//!
//! Phone numbers, fax numbers and street addresses. The staff-line regex stops at the first `<`, so
//! `<p>Phone: <a href="tel:…">…</a></p>` can never become a name, and the only school metadata this
//! adapter stores are city, enrolment and website. No athlete data exists in either source.
//!
//! # Resume
//!
//! Phases `ndhsaa_schools` / `ndhsaa_coaches` / `nsaa_schools` / `nsaa_coaches`, journal key
//! `<state>:<provider-key>` (`ND:1045`, `NE:Adams Central`). Both halves check the journal
//! **before** the per-school GET, so an interrupted walk re-fetches nothing; Nebraska's form GET is
//! served from the fetcher's disk cache on re-runs, and `options.limit` caps how many schools a
//! smoke run processes without disturbing the journal of a full walk.

use crate::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachId, CoachRole, Evidence, Gender,
    SchoolId, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use crate::net::FetchOptions;
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{Context, Result};
use regex::Regex;
use std::borrow::Cow;
use std::collections::HashSet;
use std::sync::LazyLock;

/// NDHSAA member-school index (server-rendered HTML, 169 schools, no pagination).
pub const ND_SCHOOLS_URL: &str = "https://ndhsaa.com/schools";
/// NDHSAA per-school page prefix; a school page is `<prefix><id>/<slug>`.
pub const ND_SCHOOL_BASE: &str = "https://ndhsaa.com/schools/";
/// NSAA school-directory export screen. A bare GET returns the request form **plus the option list
/// of all 312 member schools**; `?session=&school=<name>` returns one school's full record.
///
/// The screen's own bulk form (`session= `, `school=View all schools`, `submit=See School Info`)
/// renders all 312 schools into one 1,085,584-byte response, but takes **49.4 s** to do it
/// (measured 2026-09-20T14:34:47Z, HTTP 200) while the fetcher's client timeout is 45 s — the crate
/// therefore never receives that body. The per-school route answers in ~0.3 s and journals per
/// school, so it is what this adapter walks.
pub const NSAA_FORM_URL: &str = "https://secure.nsaahome.org/nsaaforms/direxportscreen.php";

/// Adapter id recorded in evidence for the North Dakota half.
const ND_ADAPTER_ID: &str = "ndhsaa";
/// Adapter id recorded in evidence for the Nebraska half.
const NSAA_ADAPTER_ID: &str = "nsaa";
/// Adapter id for the combined report.
const ADAPTER_ID: &str = "plain_names";

const ND_SCHOOLS_PHASE: &str = "ndhsaa_schools";
const ND_COACHES_PHASE: &str = "ndhsaa_coaches";
const NSAA_SCHOOLS_PHASE: &str = "nsaa_schools";
const NSAA_COACHES_PHASE: &str = "nsaa_coaches";

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

// ---------------------------------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------------------------------

static TAG_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<[^>]*>").expect("tag regex"));
/// HTML comments can hide markup, including a commented-out `<h1>` ahead of the real heading.
static COMMENT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<!--.*?-->").expect("comment regex"));
static WHITESPACE_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s+").expect("whitespace regex"));
/// Email probe: used only to *count* addresses for the honest-field note, never to store them.
static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").expect("email probe regex")
});
/// A co-op annotation is published in several shapes: `(Co-op w/Litchfield)`, a bare
/// `Co-op w/Wheeler Central`, and the source's own typo `(Co-oop w/ Loup County` without a closing
/// parenthesis. All of them start at `co-?o+p` and run to the end of the cell's value.
static COOP_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\s*(?:\(\s*)?co-?o+p\b.*$").expect("co-op regex"));
static NAME_SPLIT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s*[,/&]\s*").expect("name split regex"));

/// Office and building staff that must never become a coach or athletic director, even when the
/// label also contains "director". Matched case-insensitively against the published label.
const OFFICE_ROLE_TOKENS: [&str; 12] = [
    "secretary",
    "administrative assistant",
    "trainer",
    "principal",
    "superintendent",
    "business manager",
    "tech director",
    "technology director",
    "custodian",
    "counselor",
    "board president",
    "staff",
];

/// Remove HTML comments before matching; borrows (no copy) when the page has none.
fn without_comments(html: &str) -> Cow<'_, str> {
    COMMENT_REGEX.replace_all(html, " ")
}

/// Strip tags, decode the entities these two sources publish and collapse whitespace.
fn clean_text(raw: &str) -> String {
    let untagged = TAG_REGEX.replace_all(raw, " ");
    // `&amp;` must be decoded last so `&amp;lt;` cannot become a tag.
    let decoded = untagged
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&");
    WHITESPACE_REGEX
        .replace_all(&decoded, " ")
        .trim()
        .to_string()
}

/// Convert a non-empty trimmed string into `Some`, or `None`.
fn nonempty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Strip leading honorifics so "Mr. Barry Mink", "Coach B. Mink" and "Barry Mink" mint one coach.
fn strip_honorific(value: &str) -> String {
    let mut parts: Vec<&str> = value.split_whitespace().collect();
    while let Some(first) = parts.first() {
        let token = first.trim_end_matches('.').to_ascii_lowercase();
        if matches!(
            token.as_str(),
            "mr" | "mrs" | "ms" | "miss" | "dr" | "coach" | "sir" | "rev" | "fr"
        ) {
            parts.remove(0);
        } else {
            break;
        }
    }
    if parts.is_empty() {
        value.trim().to_string()
    } else {
        parts.join(" ")
    }
}

/// Drop any co-op annotation and then the text that carried it.
fn strip_coop_note(value: &str) -> String {
    COOP_REGEX.replace(value, " ").trim().to_string()
}

/// Split a published name cell into person names: `,`, `/` and `&` all appear as separators, and
/// duplicates inside one cell (the source publishes `Jeff Tescher, Jeff Tescher`) collapse.
fn split_person_names(value: &str) -> Vec<String> {
    let stripped = strip_coop_note(value);
    let mut names: Vec<String> = Vec::new();
    for part in NAME_SPLIT_REGEX.split(&stripped) {
        let name = strip_honorific(part);
        if name.is_empty() {
            continue;
        }
        if names
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(&name))
        {
            continue;
        }
        names.push(name);
    }
    names
}

/// True when a published role label is office/building staff rather than a coaching role.
fn is_office_role(lowered_label: &str) -> bool {
    OFFICE_ROLE_TOKENS
        .iter()
        .any(|token| lowered_label.contains(token))
}

/// Evidence date: `options.observed_on` when set, else the run's own date.
fn observed_on(ctx: &AdapterContext<'_>, options: &Options) -> String {
    match nonempty(&options.observed_on) {
        Some(date) => date,
        None => ctx.observed_on.clone(),
    }
}

/// Fetch options for this adapter: the context's options, plus `options.refresh` when set.
fn fetch_options(ctx: &AdapterContext<'_>, options: &Options) -> FetchOptions {
    let mut fetch = ctx.fetch_options();
    fetch.refresh = options.refresh || ctx.refresh;
    fetch
}

// ---------------------------------------------------------------------------------------------------
// North Dakota (NDHSAA)
// ---------------------------------------------------------------------------------------------------

/// One member school as listed on the NDHSAA school index.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NdSchoolRef {
    /// Numeric NDHSAA school id — stable across seasons; the slug is cosmetic.
    pub id: String,
    /// URL slug, e.g. `west-fargo-sheyenne`.
    pub slug: String,
}

impl NdSchoolRef {
    /// Canonical per-school page URL.
    pub fn url(&self) -> String {
        format!("{ND_SCHOOL_BASE}{}/{}", self.id, self.slug)
    }
}

/// A published staff line: `<p>Role: Name</p>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NdStaffRole {
    /// Published role label, e.g. `Athletic Director`.
    pub label: String,
    /// Published person name, honorifics intact.
    pub name: String,
}

/// One row of the NDHSAA "Sport/Activity Offering | Coaches" table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NdOffering {
    /// Sport label with the co-op annotation removed, e.g. `Boys' Cross Country`.
    pub label: String,
    /// Coach names as published: separators split, honorifics stripped, duplicates collapsed.
    pub coaches: Vec<String>,
    /// Co-op annotation from the sport cell, e.g. `Some("Mandan")`.
    pub co_op: Option<String>,
}

static ND_LINK_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"href="(?:https?://ndhsaa\.com)?/schools/(\d+)/([a-z0-9\-]+)""#)
        .expect("ndhsaa school link regex")
});
static ND_HEADING_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<h1[^>]*>(.*?)</h1>").expect("ndhsaa heading regex"));
static ND_STAFF_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<p>\s*([A-Za-z][^:<]{1,48}?)\s*:\s*([^<]*)</p>").expect("ndhsaa staff regex")
});
static ND_ROW_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)<tr[^>]*>\s*<td class="p-2">\s*(.*?)\s*</td>\s*<td class="p-2">\s*(.*?)\s*</td>\s*</tr>"#,
    )
    .expect("ndhsaa coach-row regex")
});
static ND_COOP_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\(\s*coop:\s*([^)]+)\)").expect("ndhsaa coop regex"));
static ND_ADDRESS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<p>\s*Address:\s*([^<]*)</p>").expect("ndhsaa address regex")
});
static ND_ENROLLMENT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)([\d,]+)\s+students enrolled").expect("ndhsaa enrollment regex")
});
static ND_WEBSITE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?s)<p>\s*Website:\s*<a[^>]*href="([^"]+)""#).expect("ndhsaa website regex")
});

/// Parse the member-school index, keeping the first link per numeric id.
///
/// The index is server-rendered and paginates nothing: one GET yields all 169 schools. Both the
/// absolute (`https://ndhsaa.com/schools/…`) and the relative (`/schools/…`) link forms are handled.
pub fn parse_nd_school_refs(html: &str) -> Vec<NdSchoolRef> {
    let html = without_comments(html);
    let html: &str = &html;
    let mut members: Vec<NdSchoolRef> = Vec::new();
    let mut seen: HashSet<&str> = HashSet::new();
    for capture in ND_LINK_REGEX.captures_iter(html) {
        let (Some(id), Some(slug)) = (capture.get(1), capture.get(2)) else {
            continue;
        };
        if !seen.insert(id.as_str()) {
            continue;
        }
        members.push(NdSchoolRef {
            id: id.as_str().to_string(),
            slug: slug.as_str().to_string(),
        });
    }
    members
}

/// City from the `Address:` line: `800 40th Ave E., West Fargo, ND 58078` → `West Fargo`.
///
/// The line is read whole and cut at the trailing `, ND …`, then the last comma segment before the
/// state is the city. Matching a bare `City, ND 12345` pattern against the raw HTML would instead
/// swallow part of the street (`… th Ave E., West Fargo`), because the street itself contains commas.
fn nd_city(html: &str) -> Option<String> {
    let raw = ND_ADDRESS_REGEX.captures(html)?.get(1)?.as_str();
    let address = clean_text(raw);
    let cut = address.rfind(", ND").unwrap_or(address.len());
    let city = address[..cut].rsplit(',').next()?;
    nonempty(city)
}

/// Enrolment from `Grades 9-12, 1399 students enrolled in 2025`.
fn nd_enrollment(html: &str) -> Option<u32> {
    let capture = ND_ENROLLMENT_REGEX.captures(html)?;
    let digits: String = capture
        .get(1)?
        .as_str()
        .chars()
        .filter(char::is_ascii_digit)
        .collect();
    digits.parse().ok()
}

/// Canonical school for one NDHSAA school page.
///
/// The school name is the page's single `<h1>`; a page without one (or without a usable name) yields
/// `None` rather than a school named after a URL.
pub fn parse_nd_school_page(
    html: &str,
    member: &NdSchoolRef,
    observed_on: &str,
) -> Option<(CanonicalSchool, SchoolId)> {
    let html = without_comments(html);
    let html: &str = &html;
    let name = ND_HEADING_REGEX
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|heading| clean_text(heading.as_str()))
        .filter(|name| !name.is_empty())?;

    let url = member.url();
    let (mut school, school_id) = CanonicalSchool::new("ND", &name, normalize_name(&name));
    school.city = nd_city(html);
    school.enrollment = nd_enrollment(html);
    school.school_website = ND_WEBSITE_REGEX
        .captures(html)
        .and_then(|capture| capture.get(1))
        .and_then(|href| nonempty(href.as_str()));
    school.association = Some(ND_ADAPTER_ID.to_string());
    school.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: ND_ADAPTER_ID.to_string(),
            },
            &member.id,
        )
        .with_url(&url),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new(ND_ADAPTER_ID, Some(url)),
        observed_on,
    ));
    Some((school, school_id))
}

/// Staff lines (`Superintendent`, `Principal`, `Athletic Director`, `Business Manager`, …).
pub fn parse_nd_staff(html: &str) -> Vec<NdStaffRole> {
    let html = without_comments(html);
    let html: &str = &html;
    let mut roles: Vec<NdStaffRole> = Vec::new();
    for capture in ND_STAFF_REGEX.captures_iter(html) {
        let (Some(label), Some(name)) = (capture.get(1), capture.get(2)) else {
            continue;
        };
        let label = clean_text(label.as_str());
        let name = clean_text(name.as_str());
        if label.is_empty() || name.is_empty() {
            continue;
        }
        roles.push(NdStaffRole { label, name });
    }
    roles
}

/// The coach table: one [`NdOffering`] per published row, blank coach cells included.
pub fn parse_nd_offerings(html: &str) -> Vec<NdOffering> {
    let html = without_comments(html);
    let html: &str = &html;
    let mut offerings: Vec<NdOffering> = Vec::new();
    for capture in ND_ROW_REGEX.captures_iter(html) {
        let (Some(label), Some(names)) = (capture.get(1), capture.get(2)) else {
            continue;
        };
        let label = clean_text(label.as_str());
        if label.is_empty() {
            continue;
        }
        let co_op = ND_COOP_REGEX
            .captures(&label)
            .and_then(|coop| coop.get(1))
            .and_then(|value| nonempty(&clean_text(value.as_str())));
        offerings.push(NdOffering {
            label: strip_coop_note(&label),
            coaches: split_person_names(&clean_text(names.as_str())),
            co_op,
        });
    }
    offerings
}

/// Map an NDHSAA sport label onto our ontology plus the gender side it covers.
///
/// Handles the coop-suffixed labels by ignoring everything the caller did not already strip, and
/// returns `None` for non-TF/XC offerings (`Cheerleading`, `Wrestling`, `Music - Vocal`, …).
pub fn parse_nd_sport(label: &str) -> Option<(Sport, Gender)> {
    let lowered = label.to_ascii_lowercase();
    let sport = if lowered.contains("cross country") || lowered.contains("cross-country") {
        Sport::CrossCountry
    } else if lowered.contains("indoor") && lowered.contains("track") {
        Sport::IndoorTrack
    } else if lowered.contains("track") {
        Sport::OutdoorTrack
    } else {
        return None;
    };
    let gender = if lowered.contains("girls") {
        Gender::Girls
    } else if lowered.contains("boys") {
        Gender::Boys
    } else {
        Gender::Mixed
    };
    Some((sport, gender))
}

/// The coach/AD role a published NDHSAA staff label implies, or `None` for office staff.
pub fn parse_nd_role(label: &str) -> Option<CoachRole> {
    let lowered = label.to_ascii_lowercase();
    if is_office_role(&lowered) {
        return None;
    }
    if lowered.contains("athletic director") || lowered.contains("activities director") {
        return Some(CoachRole::AthleticDirector);
    }
    None
}

/// Athletic/activities-director rows for one NDHSAA school, deduplicated by coach identity.
pub fn nd_ad_coaches(
    roles: &[NdStaffRole],
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> Vec<CanonicalCoach> {
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    let mut seen: HashSet<CoachId> = HashSet::new();
    for role in roles {
        let Some(kind) = parse_nd_role(&role.label) else {
            continue;
        };
        let name = strip_honorific(&role.name);
        if name.is_empty() {
            continue;
        }
        let mut coach = CanonicalCoach::new(school_id, name, None, Gender::Mixed, kind);
        coach.evidence.push(Evidence::parsed(
            SourceRef::new(ND_ADAPTER_ID, Some(source_url.to_string())),
            observed_on,
        ));
        if seen.insert(coach.id.clone()) {
            coaches.push(coach);
        }
    }
    coaches
}

/// Cross-country / track coach rows for one NDHSAA school.
///
/// `CoachRole::Unknown` is deliberate: the table says "Coaches" and never distinguishes a head coach
/// from an assistant, so no split is invented here.
pub fn nd_sport_coaches(
    offerings: &[NdOffering],
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> Vec<CanonicalCoach> {
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    let mut seen: HashSet<CoachId> = HashSet::new();
    for offering in offerings {
        let Some((sport, gender)) = parse_nd_sport(&offering.label) else {
            continue;
        };
        for name in &offering.coaches {
            let mut coach = CanonicalCoach::new(
                school_id,
                name.clone(),
                Some(sport),
                gender,
                CoachRole::Unknown,
            );
            coach.evidence.push(Evidence::parsed(
                SourceRef::new(ND_ADAPTER_ID, Some(source_url.to_string())),
                observed_on,
            ));
            if seen.insert(coach.id.clone()) {
                coaches.push(coach);
            }
        }
    }
    coaches
}

// ---------------------------------------------------------------------------------------------------
// Nebraska (NSAA)
// ---------------------------------------------------------------------------------------------------

/// What one NSAA directory row describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsaaRow {
    /// A sport row: the school's head coach for that sport and gender side.
    SportCoach { sport: Sport, gender: Gender },
    /// A school-wide athletic/activities-director row.
    AthleticDirector,
}

/// One published row of a school's staff/coach table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NsaaRole {
    /// Published row label, e.g. `Track & Field (Girls)` or `AD Secretary`.
    pub label: String,
    /// Published name cell, verbatim (still carrying any co-op annotation).
    pub name: String,
    /// The row carries the directory's co-op highlight (`class="table-info"`).
    pub co_op: bool,
}

/// One NSAA member school: metadata plus every published staff/coach row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NsaaSchool {
    /// Published school name — NSAA's only identity key.
    pub name: String,
    /// City from the `City, NE <zip>` line.
    pub city: Option<String>,
    /// `Enrollment:` figure.
    pub enrollment: Option<u32>,
    /// `Homepage:` link.
    pub homepage: Option<String>,
    /// Staff/coach rows in published order.
    pub roles: Vec<NsaaRole>,
}

static NSAA_BLOCK_HEADING: &str = r#"<h1 class="mt-3">"#;

static NSAA_ROW_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)<tr([^>]*)>\s*<td scope='row'>(.*?)</td>\s*<td scope='row'>(.*?)</td>\s*</tr>"#,
    )
    .expect("nsaa directory row regex")
});
static NSAA_CITY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)([A-Za-z][A-Za-z .'\-]*?)\s*,\s*NE\s+\d{5}").expect("nsaa city regex")
});
static NSAA_ENROLLMENT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)Enrollment:\s*([\d,]+)").expect("nsaa enrollment regex"));
static NSAA_HOMEPAGE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)Homepage:\s*<a[^>]*href="([^"]+)""#).expect("nsaa url regex")
});

/// The directory screen's bulk sentinel: renders every school, so it is never a school name.
const NSAA_ALL_SCHOOLS: &str = "View all schools";

static NSAA_OPTION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<option([^>]*)>(.*?)</option>").expect("nsaa option regex"));

/// One-school request URL, e.g. `…/direxportscreen.php?session=&school=Adams+Central` (form-urlencoded
/// by the `url` crate — `+` for space, hyphens literal; the server decodes both forms identically).
///
/// This is the route the adapter walks: the screen's bulk form renders all 312 schools in one body
/// and needs ~49 s to do it, while this single-school view answers in ~0.3 s.
pub fn nsaa_school_url(name: &str) -> String {
    let encoded: String = url::form_urlencoded::byte_serialize(name.as_bytes()).collect();
    format!("{NSAA_FORM_URL}?session=&school={encoded}")
}

/// The 312 member-school names from the directory form's `<option>` list.
///
/// The list carries two non-school entries — a `disabled` placeholder and the `View all schools`
/// bulk sentinel — and no `value` attributes, so the option text is the key space.
pub fn parse_nsaa_school_names(html: &str) -> Vec<String> {
    let html = without_comments(html);
    let html: &str = &html;
    let mut names: Vec<String> = Vec::new();
    for capture in NSAA_OPTION_REGEX.captures_iter(html) {
        let (Some(attributes), Some(value)) = (capture.get(1), capture.get(2)) else {
            continue;
        };
        if attributes
            .as_str()
            .to_ascii_lowercase()
            .contains("disabled")
        {
            continue;
        }
        let name = clean_text(value.as_str());
        if name.is_empty() || name == NSAA_ALL_SCHOOLS || names.contains(&name) {
            continue;
        }
        names.push(name);
    }
    names
}

/// Parse a directory response into one [`NsaaSchool`] per `<h1 class="mt-3">` block.
///
/// One school view carries a single block; the screen's bulk form carries all 312 in the same
/// markup, so the same parser serves both.
pub fn parse_nsaa_directory(html: &str) -> Vec<NsaaSchool> {
    let html = without_comments(html);
    let html: &str = &html;
    let mut schools: Vec<NsaaSchool> = Vec::new();
    for block in html.split(NSAA_BLOCK_HEADING).skip(1) {
        let Some((raw_name, rest)) = block.split_once("</h1>") else {
            continue;
        };
        let name = clean_text(raw_name);
        if name.is_empty() {
            continue;
        }
        let mut roles: Vec<NsaaRole> = Vec::new();
        for capture in NSAA_ROW_REGEX.captures_iter(rest) {
            let (Some(attributes), Some(label), Some(value)) =
                (capture.get(1), capture.get(2), capture.get(3))
            else {
                continue;
            };
            let label = clean_text(label.as_str());
            let value = clean_text(value.as_str());
            if label.is_empty() || value.is_empty() {
                continue;
            }
            roles.push(NsaaRole {
                label,
                name: value,
                co_op: attributes.as_str().contains("table-info"),
            });
        }
        let city = NSAA_CITY_REGEX
            .captures(rest)
            .and_then(|capture| capture.get(1))
            .and_then(|city| nonempty(&clean_text(city.as_str())));
        let enrollment = NSAA_ENROLLMENT_REGEX
            .captures(rest)
            .and_then(|capture| capture.get(1))
            .map(|digits| {
                digits
                    .as_str()
                    .chars()
                    .filter(char::is_ascii_digit)
                    .collect::<String>()
            })
            .and_then(|digits| digits.parse().ok());
        let homepage = NSAA_HOMEPAGE_REGEX
            .captures(rest)
            .and_then(|capture| capture.get(1))
            .and_then(|href| nonempty(href.as_str()));
        schools.push(NsaaSchool {
            name,
            city,
            enrollment,
            homepage,
            roles,
        });
    }
    schools
}

/// Classify one NSAA directory row label.
///
/// Office roles are rejected first, so an `AD Secretary` never reaches the director branch. The
/// `Unified Track & Field` activity is a distinct NSAA offering (Special Olympics unified) and is not
/// the census's track & field sport, so it is excluded as well.
pub fn parse_nsaa_row(label: &str) -> Option<NsaaRow> {
    let lowered = label.to_ascii_lowercase();
    if is_office_role(&lowered) {
        return None;
    }
    if lowered.contains("athletic director") || lowered.contains("activities director") {
        return Some(NsaaRow::AthleticDirector);
    }
    if lowered.contains("unified") {
        return None;
    }
    let sport = if lowered.contains("cross-country") || lowered.contains("cross country") {
        Sport::CrossCountry
    } else if lowered.contains("track") {
        Sport::OutdoorTrack
    } else {
        return None;
    };
    let gender = if lowered.contains("girls") {
        Gender::Girls
    } else if lowered.contains("boys") {
        Gender::Boys
    } else {
        Gender::Mixed
    };
    Some(NsaaRow::SportCoach { sport, gender })
}

/// Canonical school for one NSAA directory entry. NSAA publishes no numeric school id, so the
/// published school name is the provider key (recorded in the association-school namespace).
pub fn parse_nsaa_school(
    school: &NsaaSchool,
    source_url: &str,
    observed_on: &str,
) -> (CanonicalSchool, SchoolId) {
    let (mut canonical, school_id) =
        CanonicalSchool::new("NE", &school.name, normalize_name(&school.name));
    canonical.city = school.city.clone();
    canonical.enrollment = school.enrollment;
    canonical.school_website = school.homepage.clone();
    canonical.association = Some(NSAA_ADAPTER_ID.to_string());
    canonical.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: NSAA_ADAPTER_ID.to_string(),
            },
            &school.name,
        )
        .with_url(source_url),
    );
    canonical.evidence.push(Evidence::parsed(
        SourceRef::new(NSAA_ADAPTER_ID, Some(source_url.to_string())),
        observed_on,
    ));
    (canonical, school_id)
}

/// Coach and athletic-director rows for one NSAA school, deduplicated by coach identity.
pub fn nsaa_coaches(
    school: &NsaaSchool,
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> Vec<CanonicalCoach> {
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    let mut seen: HashSet<CoachId> = HashSet::new();
    for role in &school.roles {
        let Some(kind) = parse_nsaa_row(&role.label) else {
            continue;
        };
        for name in split_person_names(&role.name) {
            let mut coach = match kind {
                NsaaRow::SportCoach { sport, gender } => {
                    CanonicalCoach::new(school_id, name, Some(sport), gender, CoachRole::HeadCoach)
                }
                NsaaRow::AthleticDirector => CanonicalCoach::new(
                    school_id,
                    name,
                    None,
                    Gender::Mixed,
                    CoachRole::AthleticDirector,
                ),
            };
            coach.evidence.push(Evidence::parsed(
                SourceRef::new(NSAA_ADAPTER_ID, Some(source_url.to_string())),
                observed_on,
            ));
            if seen.insert(coach.id.clone()) {
                coaches.push(coach);
            }
        }
    }
    coaches
}

// ---------------------------------------------------------------------------------------------------
// Collect
// ---------------------------------------------------------------------------------------------------

/// Collect this provider's schools and coach/AD contacts into the canonical store.
///
/// `options.states` selects the half to run: `ND` the North Dakota half, `NE` the Nebraska half, and
/// an empty list both. Each half is independent, journalled separately and honours `options.limit`.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let mut report = AdapterReport::new(ADAPTER_ID, "schools");
    let before = ctx.fetcher.stats().await;
    let fetch = fetch_options(ctx, options);

    let states: Vec<String> = options
        .states
        .iter()
        .map(|state| state.trim().to_ascii_uppercase())
        .filter(|state| !state.is_empty())
        .collect();
    let run_nd = states.is_empty() || states.iter().any(|state| state == "ND");
    let run_ne = states.is_empty() || states.iter().any(|state| state == "NE");

    let mut schools_written = 0u64;
    let mut coaches_written = 0u64;
    if run_nd {
        let (schools, coaches) = collect_north_dakota(ctx, options, &fetch, &mut report).await?;
        schools_written += schools;
        coaches_written += coaches;
    }
    if run_ne {
        let (schools, coaches) = collect_nebraska(ctx, options, &fetch, &mut report).await?;
        schools_written += schools;
        coaches_written += coaches;
    }
    if !run_nd && !run_ne {
        report.note(format!(
            "no provider selected: states {:?} match neither ND nor NE",
            options.states
        ));
    }
    if !options.school_names.is_empty() {
        report.note("school_names ignored: both providers publish a bulk school index");
    }

    let after = ctx.fetcher.stats().await;
    report.requests = after.requests.saturating_sub(before.requests);
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    report.rows = schools_written;
    report.note(format!(
        "wrote {schools_written} schools and {coaches_written} coach/AD rows"
    ));
    // Neither provider publishes a coach email; every coach entity is written with `None`.
    report.with_email = 0;
    report.note("names only: provider publishes no coach email");
    Ok(report)
}

/// North Dakota half: index, then one page per member school.
async fn collect_north_dakota(
    ctx: &AdapterContext<'_>,
    options: &Options,
    fetch: &FetchOptions,
    report: &mut AdapterReport,
) -> Result<(u64, u64)> {
    let index = match ctx.fetcher.get(ND_SCHOOLS_URL, fetch).await {
        Ok(outcome) => outcome,
        Err(error) => {
            report.errors += 1;
            report.note(format!("ndhsaa: {ND_SCHOOLS_URL} failed: {error}"));
            return Ok((0, 0));
        }
    };
    let members = parse_nd_school_refs(&index.text());
    if members.is_empty() {
        report.errors += 1;
        report.note(format!(
            "ndhsaa: {ND_SCHOOLS_URL} carried no member-school links"
        ));
        return Ok((0, 0));
    }

    let observed_on = observed_on(ctx, options);
    let journal = ctx.store.journal_keys(ND_SCHOOLS_PHASE)?;
    let mut schools: Vec<CanonicalSchool> = Vec::new();
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    let mut processed = 0usize;
    let mut resumed = 0usize;
    let mut failed = 0usize;
    let mut ad_rows = 0usize;
    let mut slots = 0usize;
    let mut slots_named = 0usize;
    let mut pages_with_email = 0usize;

    for member in &members {
        if options.limit.is_some_and(|limit| processed >= limit) {
            break;
        }
        let key = format!("ND:{}", member.id);
        if journal.contains(&key) {
            resumed += 1;
            continue;
        }
        let url = member.url();
        let page = match ctx.fetcher.get(&url, fetch).await {
            Ok(outcome) => outcome,
            Err(error) => {
                failed += 1;
                report.errors += 1;
                report.note(format!("ndhsaa: {url} failed: {error}"));
                continue;
            }
        };
        let html = page.text();
        let Some((school, school_id)) = parse_nd_school_page(&html, member, &observed_on) else {
            failed += 1;
            report.errors += 1;
            report.note(format!("ndhsaa: {url} carried no school heading"));
            continue;
        };

        if EMAIL_REGEX.is_match(&html) {
            pages_with_email += 1;
        }
        let staff = parse_nd_staff(&html);
        let offerings = parse_nd_offerings(&html);
        let ads = nd_ad_coaches(&staff, &school_id, &url, &observed_on);
        let sport_coaches = nd_sport_coaches(&offerings, &school_id, &url, &observed_on);
        for offering in &offerings {
            if parse_nd_sport(&offering.label).is_some() {
                slots += 1;
                if !offering.coaches.is_empty() {
                    slots_named += 1;
                }
            }
        }
        ad_rows += ads.len();
        let ad_count = ads.len();
        let sport_count = sport_coaches.len();
        coaches.extend(ads);
        coaches.extend(sport_coaches);
        schools.push(school);

        ctx.store.journal_done(
            ND_SCHOOLS_PHASE,
            &key,
            &serde_json::json!({ "slug": member.slug, "offerings": offerings.len() }),
        )?;
        ctx.store.journal_done(
            ND_COACHES_PHASE,
            &key,
            &serde_json::json!({ "coach_rows": sport_count + ad_count, "ad_rows": ad_count }),
        )?;
        processed += 1;
    }

    let school_rows = schools.len() as u64;
    let coach_rows = coaches.len() as u64;
    ctx.store
        .append_many(Table::Schools, &schools)
        .context("writing ndhsaa schools")?;
    ctx.store
        .append_many(Table::Coaches, &coaches)
        .context("writing ndhsaa coaches")?;

    report.note(format!(
        "ndhsaa: {processed} of {} member schools parsed ({resumed} already journalled, {failed} failed); \
         {coach_rows} coach rows ({ad_rows} athletic/activities directors); \
         TF/XC coach slots named {slots_named}/{slots}; {} listings",
        members.len(),
        members.len()
    ));
    report.note(format!(
        "ndhsaa: {pages_with_email} of {processed} school pages walked contain any email string; \
         names only: provider publishes no coach email (every coach entity has professional_email=None)"
    ));
    Ok((school_rows, coach_rows))
}

/// Nebraska half: the directory form yields the 312 member names, then one GET per school.
async fn collect_nebraska(
    ctx: &AdapterContext<'_>,
    options: &Options,
    fetch: &FetchOptions,
    report: &mut AdapterReport,
) -> Result<(u64, u64)> {
    let form = match ctx.fetcher.get(NSAA_FORM_URL, fetch).await {
        Ok(outcome) => outcome,
        Err(error) => {
            report.errors += 1;
            report.note(format!("nsaa: {NSAA_FORM_URL} failed: {error}"));
            return Ok((0, 0));
        }
    };
    let members = parse_nsaa_school_names(&form.text());
    if members.is_empty() {
        report.errors += 1;
        report.note(format!(
            "nsaa: {NSAA_FORM_URL} carried no member-school options"
        ));
        return Ok((0, 0));
    }

    let observed_on = observed_on(ctx, options);
    let journal = ctx.store.journal_keys(NSAA_SCHOOLS_PHASE)?;
    let mut schools: Vec<CanonicalSchool> = Vec::new();
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    let mut processed = 0usize;
    let mut resumed = 0usize;
    let mut failed = 0usize;
    let mut ad_rows = 0usize;
    let mut sport_rows = 0usize;
    let mut slots = 0usize;
    let mut slots_named = 0usize;
    let mut rows_with_email = 0usize;
    let mut coach_rows_with_email = 0usize;
    let mut role_rows = 0usize;

    for name in &members {
        if options.limit.is_some_and(|limit| processed >= limit) {
            break;
        }
        let key = format!("NE:{name}");
        if journal.contains(&key) {
            resumed += 1;
            continue;
        }
        let url = nsaa_school_url(name);
        let page = match ctx.fetcher.get(&url, fetch).await {
            Ok(outcome) => outcome,
            Err(error) => {
                failed += 1;
                report.errors += 1;
                report.note(format!("nsaa: {url} failed: {error}"));
                continue;
            }
        };
        let blocks = parse_nsaa_directory(&page.text());
        let Some(entry) = blocks
            .iter()
            .find(|entry| &entry.name == name)
            .or_else(|| blocks.first())
        else {
            failed += 1;
            report.errors += 1;
            report.note(format!("nsaa: {url} carried no school block"));
            continue;
        };

        let (school, school_id) = parse_nsaa_school(entry, &url, &observed_on);
        let school_coaches = nsaa_coaches(entry, &school_id, &url, &observed_on);
        for role in &entry.roles {
            role_rows += 1;
            if EMAIL_REGEX.is_match(&role.name) {
                rows_with_email += 1;
                if parse_nsaa_row(&role.label).is_some() {
                    coach_rows_with_email += 1;
                }
            }
            if matches!(
                parse_nsaa_row(&role.label),
                Some(NsaaRow::SportCoach { .. })
            ) {
                slots += 1;
                if !split_person_names(&role.name).is_empty() {
                    slots_named += 1;
                }
            }
        }
        ad_rows += school_coaches
            .iter()
            .filter(|coach| coach.role == CoachRole::AthleticDirector)
            .count();
        sport_rows += school_coaches
            .iter()
            .filter(|coach| coach.role == CoachRole::HeadCoach)
            .count();
        let coach_count = school_coaches.len();
        coaches.extend(school_coaches);
        schools.push(school);

        ctx.store.journal_done(
            NSAA_SCHOOLS_PHASE,
            &key,
            &serde_json::json!({ "published_rows": entry.roles.len() }),
        )?;
        ctx.store.journal_done(
            NSAA_COACHES_PHASE,
            &key,
            &serde_json::json!({ "coach_rows": coach_count }),
        )?;
        processed += 1;
    }

    let school_rows = schools.len() as u64;
    let coach_rows = coaches.len() as u64;
    ctx.store
        .append_many(Table::Schools, &schools)
        .context("writing nsaa schools")?;
    ctx.store
        .append_many(Table::Coaches, &coaches)
        .context("writing nsaa coaches")?;

    report.note(format!(
        "nsaa: {processed} of {} member schools parsed ({resumed} already journalled, {failed} failed); \
         {coach_rows} coach rows ({ad_rows} athletic/activities directors, {sport_rows} sport rows); \
         TF/XC coach slots named {slots_named}/{slots}",
        members.len()
    ));
    report.note(format!(
        "nsaa: {rows_with_email} of {role_rows} directory rows carry an email string \
         ({coach_rows_with_email} of them in coach/AD rows); \
         names only: provider publishes no coach email"
    ));
    Ok((school_rows, coach_rows))
}

// ---------------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Fixture provenance — every file is a real capture (or, for `ND_PAGE_NO_AD`, one documented
    /// deletion from one).
    ///
    /// * `ND_INDEX` — `GET https://ndhsaa.com/schools`, HTTP 200, capture
    ///   `tools/a29-coach/nd-schools.html` (2026-09-19 23:23 CDT), 169 member-school anchors.
    /// * `ND_PAGE` — `GET https://ndhsaa.com/schools/1045/west-fargo-sheyenne`, HTTP 200, capture
    ///   `tools/a29-coach/nd-1045-west-fargo-sheyenne.html` (2026-09-19 23:24 CDT).
    /// * `ND_PAGE_NO_AD` — `GET https://ndhsaa.com/schools/1378/mandan-classical-academy`, live GET
    ///   HTTP 200 2026-09-20T14:23:43Z, minus the one `Athletic Director: …` paragraph: the provider
    ///   publishes an AD for every sampled member school (33/33 in research reports 25 & 37, 4/4 in
    ///   this session's live probes), so the no-AD shape is reproduced by deleting that line.
    /// * `NSAA_PAGE` — the directory screen's bulk POST (`session= `, `school=View all schools`,
    ///   `submit=See School Info`), live HTTP 200 1,085,584 B 2026-09-20T14:24:47Z (byte-identical to
    ///   `tools/ne/nsaa_directory_all_2026-09-19.html`), sliced to its first 8 school blocks. The same
    ///   `NsaaSchool` markup appears one school at a time in `NSAA_SCHOOL_GET`.
    /// * `NSAA_FORM` — `GET https://secure.nsaahome.org/nsaaforms/direxportscreen.php`, live HTTP 200
    ///   11,585 B 2026-09-20T14:36:10Z: the request form plus the 314 `<option>` entries (312 schools
    ///   + a disabled placeholder + the "View all schools" sentinel).
    /// * `NSAA_SCHOOL_GET` — `GET …?session=&school=Adams%20Central`, live HTTP 200 15,579 B
    ///   2026-09-20T14:36:12Z: one school, one `<h1 class="mt-3">` block, the same 34 rows as the
    ///   bulk block for Adams Central.
    const ND_INDEX: &str = include_str!("../../tests/fixtures/plain_names/nd_schools_index.html");
    const ND_PAGE: &str = include_str!("../../tests/fixtures/plain_names/nd_school_page.html");
    const ND_PAGE_NO_AD: &str =
        include_str!("../../tests/fixtures/plain_names/nd_school_page_no_ad.html");
    const NSAA_PAGE: &str =
        include_str!("../../tests/fixtures/plain_names/nsaa_directory_export.html");
    const NSAA_FORM: &str =
        include_str!("../../tests/fixtures/plain_names/nsaa_directory_form.html");
    const NSAA_SCHOOL_GET: &str =
        include_str!("../../tests/fixtures/plain_names/nsaa_school_get_adams_central.html");

    const OBSERVED_ON: &str = "2026-09-20";

    fn sheyenne() -> NdSchoolRef {
        NdSchoolRef {
            id: "1045".to_string(),
            slug: "west-fargo-sheyenne".to_string(),
        }
    }

    fn mandan_classical() -> NdSchoolRef {
        NdSchoolRef {
            id: "1378".to_string(),
            slug: "mandan-classical-academy".to_string(),
        }
    }

    /// The request URL the adapter would have used for a fixture school: evidence always cites the
    /// per-school request that produced the row, never the directory form that listed it.
    fn url_of(school: &NsaaSchool) -> String {
        nsaa_school_url(&school.name)
    }

    /// Every string field of an entity, so a test can prove a value cannot leak from *any* field.
    fn serialized<T: serde::Serialize>(value: &T) -> String {
        serde_json::to_string(value).expect("entity serializes")
    }

    // ------------------------------ North Dakota ------------------------------

    #[test]
    fn nd_index_lists_every_member_school() {
        let members = parse_nd_school_refs(ND_INDEX);
        assert_eq!(
            members.len(),
            169,
            "the index carries all 169 member schools"
        );

        let ids: HashSet<&str> = members.iter().map(|member| member.id.as_str()).collect();
        assert_eq!(ids.len(), members.len(), "ids are unique after dedupe");
        assert!(ids.contains("1045"));

        let sheyenne = members
            .iter()
            .find(|member| member.id == "1045")
            .expect("West Fargo Sheyenne is a member");
        assert_eq!(sheyenne.slug, "west-fargo-sheyenne");
        assert_eq!(
            sheyenne.url(),
            "https://ndhsaa.com/schools/1045/west-fargo-sheyenne"
        );
    }

    #[test]
    fn nd_index_dedupes_and_ignores_other_links() {
        let html = r#"
            <a href="https://ndhsaa.com/schools/7/bismarck">Bismarck</a>
            <a href="/schools/7/bismarck-high">Bismarck again</a>
            <a href="/schools">All schools</a>
            <a href="https://ndhsaa.com/athletics/track-boys">Track</a>
            <a href="/schools/92/alexander">Alexander</a>
        "#;
        let members = parse_nd_school_refs(html);
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].slug, "bismarck", "first link per id wins");
        assert_eq!(members[1].id, "92");
    }

    #[test]
    fn nd_school_page_parses_school_metadata() {
        let (school, school_id) =
            parse_nd_school_page(ND_PAGE, &sheyenne(), OBSERVED_ON).expect("fixture has a heading");

        assert_eq!(school.name, "West Fargo Sheyenne High School");
        assert_eq!(
            school.normalized_name, "west fargo sheyenne",
            "normalize_name drops the `High School` suffix"
        );
        assert_eq!(school.state.as_deref(), Some("ND"));
        assert_eq!(school.association.as_deref(), Some("ndhsaa"));
        assert_eq!(school.city.as_deref(), Some("West Fargo"));
        assert_eq!(
            school.enrollment,
            Some(1399),
            "1,399 students enrolled in 2025"
        );
        assert_eq!(
            school.school_website.as_deref(),
            Some("https://www.west-fargo.k12.nd.us/shs/activities")
        );
        assert_eq!(school.source_identities.len(), 1);
        assert_eq!(
            school.source_identities[0].namespace,
            SourceNamespace::AssociationSchool {
                association: "ndhsaa".to_string()
            }
        );
        assert_eq!(school.source_identities[0].id, "1045");
        assert_eq!(
            school.source_identities[0].url.as_deref(),
            Some("https://ndhsaa.com/schools/1045/west-fargo-sheyenne")
        );
        assert_eq!(school.evidence.len(), 1);
        assert_eq!(school.evidence[0].observed_on, OBSERVED_ON);
        assert_eq!(school.evidence[0].source.id, "ndhsaa");
        assert_eq!(
            school.evidence[0].source.url.as_deref(),
            Some("https://ndhsaa.com/schools/1045/west-fargo-sheyenne")
        );
        assert_eq!(
            school_id,
            CanonicalSchool::mint("ND", &school.name, &school.normalized_name)
        );
        // Identity is the natural key, not the display name: the same school published elsewhere as
        // "West Fargo Sheyenne HS" mints the identical id, so the two observations merge.
        let (_, same_school) = CanonicalSchool::new(
            "ND",
            "West Fargo Sheyenne HS",
            normalize_name("West Fargo Sheyenne HS"),
        );
        assert_eq!(school_id, same_school);
    }

    #[test]
    fn nd_school_page_never_stores_phone_fax_or_address() {
        let (school, _) = parse_nd_school_page(ND_PAGE, &sheyenne(), OBSERVED_ON).expect("school");
        let json = serialized(&school);
        assert!(
            !json.contains("356-2160"),
            "the school phone number is not stored"
        );
        assert!(!json.contains("499-6687"), "the fax number is not stored");
        assert!(
            !json.contains("800 40th Ave E."),
            "the street address is not stored"
        );
    }

    #[test]
    fn nd_ad_coaches_include_ad_and_activities_director_only() {
        let (_, school_id) =
            parse_nd_school_page(ND_PAGE, &sheyenne(), OBSERVED_ON).expect("school");
        let staff = parse_nd_staff(ND_PAGE);
        let coaches = nd_ad_coaches(
            &staff,
            &school_id,
            "https://ndhsaa.com/schools/1045/west-fargo-sheyenne",
            OBSERVED_ON,
        );

        let names: Vec<&str> = coaches.iter().map(|coach| coach.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["Logan Midthun", "James Moe", "Corissa Kolesar"],
            "Athletic Director and Activities Director rows collapse to one entity per person"
        );
        for coach in &coaches {
            assert_eq!(coach.role, CoachRole::AthleticDirector);
            assert_eq!(coach.sport, None, "directors are school-wide roles");
            assert_eq!(coach.gender, Gender::Mixed);
            assert_eq!(coach.professional_email, None);
            assert_eq!(coach.phone, None);
            assert_eq!(coach.evidence.len(), 1);
            assert_eq!(coach.evidence[0].observed_on, OBSERVED_ON);
        }

        // The office roles are on the page and were parsed as staff lines — they are simply never
        // promoted to coaches, however senior the person is.
        let labels: Vec<&str> = staff.iter().map(|role| role.label.as_str()).collect();
        for office in [
            "Superintendent",
            "Assistant Superintendent",
            "Principal",
            "Vice/Assistant Principal",
            "Business Manager",
            "Tech Director",
        ] {
            assert!(
                labels.contains(&office),
                "{office} is a published staff line"
            );
        }
        for (label, person) in [
            ("Superintendent", "Beth Slette"),
            ("Assistant Superintendent", "Vincent Williams"),
            ("Principal", "Ryan Salisbury"),
            ("Vice/Assistant Principal", "Ryan Bodell"),
            ("Business Manager", "Levi Bachmeier"),
            ("Tech Director", "Ed Mitchell"),
        ] {
            assert!(
                !names.contains(&person),
                "{person} ({label}) must never be emitted as a coach or director"
            );
            assert!(
                parse_nd_role(label).is_none(),
                "{label} is not a director role"
            );
        }
    }

    #[test]
    fn nd_page_without_ad_yields_no_director_rows() {
        let (school, school_id) =
            parse_nd_school_page(ND_PAGE_NO_AD, &mandan_classical(), OBSERVED_ON)
                .expect("the page still yields a school");
        assert_eq!(school.name, "Mandan Classical Academy");
        assert_eq!(school.city.as_deref(), Some("Mandan"));
        assert_eq!(school.association.as_deref(), Some("ndhsaa"));

        let staff = parse_nd_staff(ND_PAGE_NO_AD);
        assert!(
            !staff
                .iter()
                .any(|role| role.label.to_ascii_lowercase().contains("director")),
            "fixture really carries no director line"
        );
        assert!(
            staff
                .iter()
                .any(|role| role.label == "Superintendent" && role.name == "Thomas Hoopes"),
            "the superintendent is published on this page"
        );

        let coaches = nd_ad_coaches(
            &staff,
            &school_id,
            "https://ndhsaa.com/schools/1378/mandan-classical-academy",
            OBSERVED_ON,
        );
        assert!(
            coaches.is_empty(),
            "a superintendent is not an athletic director, however senior"
        );
    }

    #[test]
    fn nd_offering_rows_and_coop_annotations_parse() {
        let offerings = parse_nd_offerings(ND_PAGE);
        assert_eq!(offerings.len(), 28, "one row per published offering");

        let cross_country = offerings
            .iter()
            .find(|offering| offering.label == "Boys' Cross Country")
            .expect("Boys' Cross Country is offered");
        assert_eq!(cross_country.coaches, vec!["Troy Thorson", "Jared Slinde"]);
        assert_eq!(cross_country.co_op, None);

        let hockey = offerings
            .iter()
            .find(|offering| offering.label == "Boys' Ice Hockey")
            .expect("Boys' Ice Hockey is offered");
        assert_eq!(
            hockey.co_op.as_deref(),
            Some("West Fargo Sheyenne"),
            "the co-op annotation is recorded, not left inside the sport label"
        );

        let blank = offerings
            .iter()
            .find(|offering| offering.label == "Debate")
            .expect("Debate is offered");
        assert!(
            blank.coaches.is_empty(),
            "a blank coach cell yields no names"
        );
    }

    #[test]
    fn nd_sport_labels_map_to_sport_and_gender() {
        assert_eq!(
            parse_nd_sport("Boys' Cross Country"),
            Some((Sport::CrossCountry, Gender::Boys))
        );
        assert_eq!(
            parse_nd_sport("Girls' Cross Country"),
            Some((Sport::CrossCountry, Gender::Girls))
        );
        assert_eq!(
            parse_nd_sport("Boys' Track and Field"),
            Some((Sport::OutdoorTrack, Gender::Boys))
        );
        assert_eq!(
            parse_nd_sport("Girls' Track and Field"),
            Some((Sport::OutdoorTrack, Gender::Girls))
        );
        assert_eq!(
            parse_nd_sport("Boys' Indoor Track"),
            Some((Sport::IndoorTrack, Gender::Boys))
        );
        for other in [
            "Cheer - Boys' Basketball",
            "Volleyball",
            "Music - Vocal",
            "Student Congress",
            "Girls' Wrestling",
        ] {
            assert_eq!(
                parse_nd_sport(other),
                None,
                "{other} is not a TF/XC offering"
            );
        }
    }

    #[test]
    fn nd_sport_coaches_are_names_only() {
        let (_, school_id) =
            parse_nd_school_page(ND_PAGE, &sheyenne(), OBSERVED_ON).expect("school");
        let offerings = parse_nd_offerings(ND_PAGE);
        let coaches = nd_sport_coaches(
            &offerings,
            &school_id,
            "https://ndhsaa.com/schools/1045/west-fargo-sheyenne",
            OBSERVED_ON,
        );

        // 4 TF/XC offerings: 2+2 XC names, 1 boys TF name, 1 girls TF name = 6 entities.
        assert_eq!(coaches.len(), 6);
        for coach in &coaches {
            assert_eq!(
                coach.role,
                CoachRole::Unknown,
                "NDHSAA publishes no head/assistant split"
            );
            assert!(coach.sport.is_some());
            assert_eq!(coach.professional_email, None);
            assert_eq!(coach.phone, None);
        }
        let girls_track = coaches
            .iter()
            .find(|coach| coach.sport == Some(Sport::OutdoorTrack) && coach.gender == Gender::Girls)
            .expect("girls track coach");
        assert_eq!(girls_track.name, "Jaime Watson");
        assert!(
            coaches.iter().all(|coach| {
                !coach.evidence.is_empty() && coach.evidence[0].source.id == "ndhsaa"
            }),
            "every coach cites the page it came from"
        );
        // Non-TF/XC offerings contribute nothing.
        assert!(!coaches.iter().any(|coach| coach.name == "Tim Brandt"));
    }

    #[test]
    fn nd_fixture_reports_its_own_fill_rate() {
        let offerings = parse_nd_offerings(ND_PAGE);
        let slots: Vec<&NdOffering> = offerings
            .iter()
            .filter(|offering| parse_nd_sport(&offering.label).is_some())
            .collect();
        let named = slots
            .iter()
            .filter(|offering| !offering.coaches.is_empty())
            .count();
        assert_eq!(
            slots.len(),
            4,
            "the fixture page offers boys/girls XC and track"
        );
        assert_eq!(
            named, 4,
            "all four TF/XC coach slots are named on this page"
        );
    }

    #[test]
    fn nd_malformed_input_yields_no_rows() {
        assert!(parse_nd_school_refs("<html>no links here</html>").is_empty());
        assert!(parse_nd_school_refs("").is_empty());
        assert!(parse_nd_school_page(
            "<html><body>Nothing</body></html>",
            &sheyenne(),
            OBSERVED_ON
        )
        .is_none());
        assert!(parse_nd_staff("not html at all").is_empty());
        assert!(parse_nd_offerings("<table><tr><td>only one cell</td></tr></table>").is_empty());
        // An empty heading is not a school name.
        assert!(parse_nd_school_page(
            "<h1>   </h1><p>Address: x, Fargo, ND 58102</p>",
            &sheyenne(),
            OBSERVED_ON
        )
        .is_none());
    }

    // ------------------------------ Nebraska ------------------------------

    #[test]
    fn nsaa_form_option_list_yields_the_member_schools() {
        let names = parse_nsaa_school_names(NSAA_FORM);
        assert_eq!(names.len(), 312, "the form lists every member school");
        assert_eq!(names[0], "Adams Central");
        assert_eq!(names[1], "Ainsworth");
        assert_eq!(names[311], "Yutan");
        assert!(
            !names.iter().any(|name| name == NSAA_ALL_SCHOOLS),
            "the bulk sentinel is not a school"
        );
        assert!(
            !names.iter().any(|name| name.contains("Select a school")),
            "the disabled placeholder is not a school"
        );
        assert_eq!(
            names
                .iter()
                .filter(|name| name.as_str() == "Adams Central")
                .count(),
            1,
            "names are unique"
        );

        // The form's option names are exactly the names the bulk response uses for its blocks.
        let bulk: Vec<String> = parse_nsaa_directory(NSAA_PAGE)
            .into_iter()
            .map(|school| school.name)
            .collect();
        for name in &bulk {
            assert!(
                names.contains(name),
                "{name} is an option and a block heading"
            );
        }

        // Malformed and empty payloads yield no rows rather than panicking.
        assert!(parse_nsaa_school_names("").is_empty());
        assert!(parse_nsaa_school_names("<select></select>").is_empty());
        assert!(parse_nsaa_school_names("<option></option>").is_empty());
        assert!(
            parse_nsaa_school_names("<option disabled>Select a school to view...</option>")
                .is_empty()
        );
    }

    #[test]
    fn nsaa_school_url_round_trips_the_published_name() {
        assert_eq!(
            nsaa_school_url("Adams Central"),
            "https://secure.nsaahome.org/nsaaforms/direxportscreen.php?session=&school=Adams+Central",
            "the `+` form is what `byte_serialize` emits; the server decoded it to the same 15,579-byte \
             page as the `%20` form (live 2026-09-20T14:39:13Z, HTTP 200, byte-identical)"
        );
        assert_eq!(
            nsaa_school_url("Anselmo-Merna"),
            "https://secure.nsaahome.org/nsaaforms/direxportscreen.php?session=&school=Anselmo-Merna",
            "hyphens are safe and stay literal"
        );

        // Every published name must survive the encoding: parse the URL back and compare.
        for name in parse_nsaa_school_names(NSAA_FORM) {
            let url = url::Url::parse(&nsaa_school_url(&name)).expect("valid url");
            let decoded = url
                .query_pairs()
                .find(|(key, _)| key == "school")
                .map(|(_, value)| value.into_owned())
                .expect("school query parameter");
            assert_eq!(decoded, name, "round-trip for {name}");
        }
    }

    #[test]
    fn nsaa_single_school_page_matches_the_bulk_block() {
        let single = parse_nsaa_directory(NSAA_SCHOOL_GET);
        assert_eq!(single.len(), 1, "one school per single-school response");
        let entry = &single[0];
        assert_eq!(entry.name, "Adams Central");
        assert_eq!(entry.roles.len(), 34);
        assert_eq!(entry.city.as_deref(), Some("Hastings"));
        assert_eq!(entry.enrollment, Some(215));
        assert_eq!(
            entry.homepage.as_deref(),
            Some("http://www.adamscentral.us/")
        );

        // The same school from the bulk capture produces identical entities.
        let bulk = parse_nsaa_directory(NSAA_PAGE);
        let bulk_adams = bulk
            .iter()
            .find(|school| school.name == "Adams Central")
            .expect("Adams Central in the bulk slice");
        assert_eq!(entry.roles, bulk_adams.roles);

        let url = nsaa_school_url("Adams Central");
        let (school, school_id) = parse_nsaa_school(entry, &url, OBSERVED_ON);
        let coaches = nsaa_coaches(entry, &school_id, &url, OBSERVED_ON);
        let (_, bulk_id) = parse_nsaa_school(bulk_adams, &url, OBSERVED_ON);
        assert_eq!(school_id, bulk_id, "one canonical id either way");
        assert_eq!(coaches.len(), 6);
        assert_eq!(
            school.evidence[0].source.url.as_deref(),
            Some(nsaa_school_url("Adams Central").as_str()),
            "evidence cites the request URL that produced the row"
        );
    }

    #[test]
    fn nsaa_directory_parses_every_school_block() {
        let schools = parse_nsaa_directory(NSAA_PAGE);
        let names: Vec<&str> = schools.iter().map(|school| school.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "Adams Central",
                "Ainsworth",
                "Allen",
                "Alliance",
                "Alma",
                "Amherst",
                "Anselmo-Merna",
                "Ansley"
            ]
        );

        let adams = &schools[0];
        assert_eq!(adams.city.as_deref(), Some("Hastings"));
        assert_eq!(adams.enrollment, Some(215));
        assert_eq!(
            adams.homepage.as_deref(),
            Some("http://www.adamscentral.us/")
        );
        assert_eq!(
            adams.roles.len(),
            34,
            "one row per published staff/coach line"
        );

        let coop_rows = schools
            .iter()
            .flat_map(|school| school.roles.iter())
            .filter(|role| role.co_op)
            .count();
        assert_eq!(
            coop_rows, 32,
            "rows the directory highlights as co-op-shared carry `class='table-info'`"
        );
        assert_eq!(
            adams.roles.iter().filter(|role| role.co_op).count(),
            3,
            "Adams Central's co-op rows: Softball and the two Swimming placeholders"
        );
        let allen = schools
            .iter()
            .find(|school| school.name == "Allen")
            .expect("Allen in the fixture");
        assert_eq!(allen.roles.iter().filter(|role| role.co_op).count(), 13);
        let ansley = schools
            .iter()
            .find(|school| school.name == "Ansley")
            .expect("Ansley in the fixture");
        assert_eq!(ansley.roles.iter().filter(|role| role.co_op).count(), 10);
    }

    #[test]
    fn nsaa_sport_rows_map_to_head_coach_sport_and_gender() {
        let schools = parse_nsaa_directory(NSAA_PAGE);
        let adams = &schools[0];
        assert_eq!(
            parse_nsaa_row("Cross-Country (Boys)"),
            Some(NsaaRow::SportCoach {
                sport: Sport::CrossCountry,
                gender: Gender::Boys
            })
        );
        assert_eq!(
            parse_nsaa_row("Cross-Country (Girls)"),
            Some(NsaaRow::SportCoach {
                sport: Sport::CrossCountry,
                gender: Gender::Girls
            })
        );
        assert_eq!(
            parse_nsaa_row("Track & Field (Boys)"),
            Some(NsaaRow::SportCoach {
                sport: Sport::OutdoorTrack,
                gender: Gender::Boys
            })
        );
        assert_eq!(
            parse_nsaa_row("Track & Field (Girls)"),
            Some(NsaaRow::SportCoach {
                sport: Sport::OutdoorTrack,
                gender: Gender::Girls
            })
        );
        assert_eq!(
            parse_nsaa_row("Unified Track & Field"),
            None,
            "a distinct NSAA activity"
        );
        assert_eq!(parse_nsaa_row("Strength Coach"), None);
        assert_eq!(parse_nsaa_row("Volleyball"), None);

        let (_, school_id) = parse_nsaa_school(adams, &url_of(adams), OBSERVED_ON);
        let coaches = nsaa_coaches(adams, &school_id, &url_of(adams), OBSERVED_ON);
        let track_boys = coaches
            .iter()
            .find(|coach| {
                coach.sport == Some(Sport::OutdoorTrack)
                    && coach.gender == Gender::Boys
                    && coach.name != "Toni Fowler"
            })
            .expect("boys track coach");
        assert_eq!(track_boys.name, "Zeb Noyd");
        assert_eq!(track_boys.role, CoachRole::HeadCoach);
        let xc_boys = coaches
            .iter()
            .find(|coach| coach.sport == Some(Sport::CrossCountry) && coach.gender == Gender::Boys)
            .expect("boys XC coach");
        assert_eq!(xc_boys.name, "Toni Fowler");
        assert_eq!(xc_boys.role, CoachRole::HeadCoach);
    }

    #[test]
    fn nsaa_multi_name_cells_split_into_one_entity_per_person() {
        let schools = parse_nsaa_directory(NSAA_PAGE);
        let ainsworth = schools
            .iter()
            .find(|school| school.name == "Ainsworth")
            .expect("Ainsworth in the fixture");
        let (_, school_id) = parse_nsaa_school(ainsworth, &url_of(ainsworth), OBSERVED_ON);
        let coaches = nsaa_coaches(ainsworth, &school_id, &url_of(ainsworth), OBSERVED_ON);

        let xc: Vec<&str> = coaches
            .iter()
            .filter(|coach| coach.sport == Some(Sport::CrossCountry))
            .map(|coach| coach.name.as_str())
            .collect();
        assert_eq!(
            xc,
            vec![
                "Trey Schlueter",
                "Katie Winters",
                "Trey Schlueter",
                "Katie Winters"
            ],
            "`Trey Schlueter/Katie Winters` is two people, not one name field"
        );
        assert!(!xc.iter().any(|name| name.contains('/')));
    }

    #[test]
    fn nsaa_director_rows_map_to_athletic_director() {
        let schools = parse_nsaa_directory(NSAA_PAGE);
        let adams = &schools[0];
        let (school, school_id) = parse_nsaa_school(adams, &url_of(adams), OBSERVED_ON);
        assert_eq!(school.state.as_deref(), Some("NE"));
        assert_eq!(school.association.as_deref(), Some("nsaa"));
        assert_eq!(school.city.as_deref(), Some("Hastings"));
        assert_eq!(school.enrollment, Some(215));
        assert_eq!(school.source_identities.len(), 1);
        assert_eq!(
            school.source_identities[0].namespace,
            SourceNamespace::AssociationSchool {
                association: "nsaa".to_string()
            }
        );
        assert_eq!(
            school.source_identities[0].id, "Adams Central",
            "NSAA publishes no numeric id, so the published name is the provider key"
        );

        let coaches = nsaa_coaches(adams, &school_id, &url_of(adams), OBSERVED_ON);
        let directors: Vec<&str> = coaches
            .iter()
            .filter(|coach| coach.role == CoachRole::AthleticDirector)
            .map(|coach| coach.name.as_str())
            .collect();
        assert_eq!(
            directors,
            vec!["Alan Frank", "Aub Boucher"],
            "Activities Director + Athletic Director collapse, the assistant director is an AD row"
        );
        for coach in coaches
            .iter()
            .filter(|coach| coach.role == CoachRole::AthleticDirector)
        {
            assert_eq!(coach.sport, None);
            assert_eq!(coach.gender, Gender::Mixed);
        }
    }

    #[test]
    fn nsaa_office_roles_are_never_emitted() {
        let schools = parse_nsaa_directory(NSAA_PAGE);
        // Every one of these people is published in an office row in the fixture and appears in no
        // coach/AD row anywhere in it.
        let office_people = [
            "Shawn Scott",           // Superintendent, Adams Central
            "Scott Harrington",      // Principal, Adams Central
            "Mattison Tinant",       // AD Secretary, Adams Central
            "Dave Johnson",          // Board President, Adams Central
            "Becky Fisher",          // Guidance Counselor, Adams Central
            "Sean Vonderfecht",      // Trainer, Adams Central
            "Dale Hafer",            // Superintendent, Ainsworth
            "Kari Painter",          // AD Secretary, Ainsworth
            "Brad Wilkins",          // Board President, Ainsworth
            "Jerry Bockman",         // Trainer, Ainsworth
            "Mike Pattee",           // Superintendent, Allen
            "Chris Blohm",           // Principal, Allen
            "Becky Stapleton",       // AD Secretary, Allen
            "Jason Olesen",          // Board President, Allen
            "Kim Jonas",             // Superintendent, Ansley
            "Chrissy Slingsby",      // AD Secretary, Ansley
            "Roger Thomsen",         // Superintendent *and* Principal, Amherst
            "Carlene Abbott",        // AD Secretary, Amherst
            "Bobbi Sorensen",        // Guidance Counselor, Amherst
            "Aaron Klingelhoefer",   // Trainer, Amherst
            "Lloyd McIntyre", // Superintendent, Anselmo-Merna (also coaches Golf, not a census sport)
            "Molli Miller",   // Guidance Counselor, Anselmo-Merna
            "Dr. Troy Unzicker", // Superintendent, Alliance
            "Marissa Rotness", // AD Secretary, Alliance
            "Tim Kollars",    // Board President, Alliance
            "Tim Devlin",     // Trainer, Alliance
            "Stephanie Brandyberry", // Principal, Alma
            "Hannah Sindelar", // AD Secretary, Alma
            "Nick Simonson",  // Board President, Alma
            "Brittney Biskup", // Guidance Counselor, Alma
        ];

        let mut produced: Vec<String> = Vec::new();
        for entry in &schools {
            let (_, school_id) = parse_nsaa_school(entry, &url_of(entry), OBSERVED_ON);
            produced.extend(
                nsaa_coaches(entry, &school_id, &url_of(entry), OBSERVED_ON)
                    .into_iter()
                    .map(|coach| coach.name),
            );
        }
        for person in office_people {
            assert!(
                !produced.contains(&person.to_string()),
                "{person} works in the office, not on the track"
            );
        }

        // The labels really are published — the parser sees and rejects them.
        let labels: Vec<&str> = schools
            .iter()
            .flat_map(|school| school.roles.iter())
            .map(|role| role.label.as_str())
            .collect();
        for office in [
            "Superintendent",
            "Principal",
            "AD Secretary",
            "Trainer",
            "Board President",
            "Guidance Counselor",
            "Student Council Sponsor",
        ] {
            assert!(
                labels.contains(&office),
                "{office} is a published row label"
            );
            assert!(
                parse_nsaa_row(office).is_none(),
                "{office} is not a coach/AD row"
            );
        }
        assert!(parse_nsaa_row("Assistant Athletic Director").is_some());
        assert_eq!(parse_nsaa_row("Athletic Director Secretary"), None);
    }

    #[test]
    fn nsaa_office_row_is_ignored_but_the_same_persons_coaching_row_is_kept() {
        let schools = parse_nsaa_directory(NSAA_PAGE);

        // Alliance publishes Nate Lanik as Guidance Counselor *and* as both track coaches: the office
        // row contributes nothing, the sport rows contribute exactly two entities.
        let alliance = schools
            .iter()
            .find(|school| school.name == "Alliance")
            .expect("Alliance in the fixture");
        assert!(alliance
            .roles
            .iter()
            .any(|role| role.label == "Guidance Counselor" && role.name == "Nate Lanik"));
        let (_, alliance_id) = parse_nsaa_school(alliance, &url_of(alliance), OBSERVED_ON);
        let alliance_coaches = nsaa_coaches(alliance, &alliance_id, &url_of(alliance), OBSERVED_ON);
        assert_eq!(alliance_coaches.len(), 5, "Alliance: 1 AD + 2 XC + 2 track");
        let lanik: Vec<&CanonicalCoach> = alliance_coaches
            .iter()
            .filter(|coach| coach.name == "Nate Lanik")
            .collect();
        assert_eq!(lanik.len(), 2);
        for coach in lanik {
            assert_eq!(coach.role, CoachRole::HeadCoach);
            assert_eq!(coach.sport, Some(Sport::OutdoorTrack));
        }
        // `Unified Track & Field` also lists Nate Lanik and is not a census sport.
        assert!(alliance
            .roles
            .iter()
            .any(|role| role.label == "Unified Track & Field"));

        // Anselmo-Merna publishes Chanc McIntosh as Principal and as Activities/Athletic Director:
        // one AD entity, and no second entity from the Principal row.
        let anselmo = schools
            .iter()
            .find(|school| school.name == "Anselmo-Merna")
            .expect("Anselmo-Merna in the fixture");
        let (_, anselmo_id) = parse_nsaa_school(anselmo, &url_of(anselmo), OBSERVED_ON);
        let anselmo_coaches = nsaa_coaches(anselmo, &anselmo_id, &url_of(anselmo), OBSERVED_ON);
        assert_eq!(anselmo_coaches.len(), 3, "1 AD + 2 track");
        assert_eq!(
            anselmo_coaches
                .iter()
                .filter(|coach| coach.name == "Chanc McIntosh")
                .count(),
            1
        );

        // Ansley publishes Garrod Fernau as Principal *and* as Assistant Athletic Director: he is
        // present as a director, because a real director row names him.
        let ansley = schools
            .iter()
            .find(|school| school.name == "Ansley")
            .expect("Ansley in the fixture");
        assert!(ansley
            .roles
            .iter()
            .any(|role| role.label == "Principal" && role.name == "Garrod Fernau"));
        let (_, ansley_id) = parse_nsaa_school(ansley, &url_of(ansley), OBSERVED_ON);
        let ansley_coaches = nsaa_coaches(ansley, &ansley_id, &url_of(ansley), OBSERVED_ON);
        assert_eq!(ansley_coaches.len(), 6, "2 AD + 2 XC + 2 track");
        assert_eq!(
            ansley_coaches
                .iter()
                .filter(|coach| coach.name == "Garrod Fernau")
                .count(),
            1,
            "his Assistant Athletic Director row imports him; the Principal row does not"
        );
    }

    #[test]
    fn nsaa_fixture_yields_exactly_the_verified_entities() {
        let schools = parse_nsaa_directory(NSAA_PAGE);
        let mut by_school: Vec<(String, Vec<String>)> = Vec::new();
        for entry in &schools {
            let (_, school_id) = parse_nsaa_school(entry, &url_of(entry), OBSERVED_ON);
            let mut rows: Vec<String> =
                nsaa_coaches(entry, &school_id, &url_of(entry), OBSERVED_ON)
                    .into_iter()
                    .map(|coach| {
                        format!(
                            "{}|{:?}|{:?}|{:?}",
                            coach.name, coach.sport, coach.gender, coach.role
                        )
                    })
                    .collect();
            rows.sort();
            by_school.push((entry.name.clone(), rows));
        }

        let adams = &by_school[0];
        assert_eq!(adams.0, "Adams Central");
        assert_eq!(
            adams.1,
            vec![
                "Alan Frank|None|Mixed|AthleticDirector",
                "Aub Boucher|None|Mixed|AthleticDirector",
                "Toni Fowler|Some(CrossCountry)|Boys|HeadCoach",
                "Toni Fowler|Some(CrossCountry)|Girls|HeadCoach",
                "Toni Fowler|Some(OutdoorTrack)|Girls|HeadCoach",
                "Zeb Noyd|Some(OutdoorTrack)|Boys|HeadCoach",
            ]
        );
        let ansley = by_school
            .iter()
            .find(|(name, _)| name == "Ansley")
            .expect("Ansley");
        assert_eq!(
            ansley.1,
            vec![
                "Aaron Wagner|None|Mixed|AthleticDirector",
                "Cayley Bailey|Some(CrossCountry)|Boys|HeadCoach",
                "Cayley Bailey|Some(CrossCountry)|Girls|HeadCoach",
                "Garrod Fernau|None|Mixed|AthleticDirector",
                "Jamee Smith|Some(OutdoorTrack)|Boys|HeadCoach",
                "Jamee Smith|Some(OutdoorTrack)|Girls|HeadCoach",
            ]
        );
        let total: usize = by_school.iter().map(|(_, rows)| rows.len()).sum();
        assert_eq!(total, 42);
    }

    #[test]
    fn nsaa_coop_annotations_are_stripped_from_names() {
        let schools = parse_nsaa_directory(NSAA_PAGE);
        let ansley = schools
            .iter()
            .find(|school| school.name == "Ansley")
            .expect("Ansley in the fixture");
        assert!(
            ansley
                .roles
                .iter()
                .any(|role| role.name == "Cayley Bailey (Co-op w/Litchfield)"),
            "the raw cell text is kept on the parsed row"
        );

        let (_, school_id) = parse_nsaa_school(ansley, &url_of(ansley), OBSERVED_ON);
        let names: Vec<String> = nsaa_coaches(ansley, &school_id, &url_of(ansley), OBSERVED_ON)
            .into_iter()
            .map(|coach| coach.name)
            .collect();
        assert!(names.contains(&"Cayley Bailey".to_string()));
        assert!(names.contains(&"Jamee Smith".to_string()));
        assert!(!names.iter().any(|name| name.contains("Co-op")));

        // The other shapes the source publishes, verbatim from the 2026-09-20 full capture.
        assert_eq!(
            split_person_names("Cayley Bailey (Co-op w/Litchfield)"),
            vec!["Cayley Bailey"]
        );
        assert_eq!(
            split_person_names("Derek Mahony Co-op w/Wheeler Central"),
            vec!["Derek Mahony"]
        );
        assert_eq!(
            split_person_names("Jenna Landgren Co-op w/Wheeler Central"),
            vec!["Jenna Landgren"]
        );
        assert_eq!(
            split_person_names("Carrie Ourada (Co-oop w/ Loup County"),
            vec!["Carrie Ourada"]
        );
        assert!(split_person_names("Co-op w/Loup CIty").is_empty());
        assert!(split_person_names("   ").is_empty());
        assert_eq!(
            split_person_names("Betsy Rall & Amy Sokol"),
            vec!["Betsy Rall", "Amy Sokol"]
        );
        assert_eq!(
            split_person_names("Jeff Tescher, Jeff Tescher"),
            vec!["Jeff Tescher"]
        );
        assert_eq!(split_person_names("Dr. Dan Schinzel"), vec!["Dan Schinzel"]);
    }

    #[test]
    fn nsaa_fixture_reports_its_own_fill_rate_and_entity_count() {
        let schools = parse_nsaa_directory(NSAA_PAGE);
        let mut slots = 0usize;
        let mut named = 0usize;
        let mut coaches = 0usize;
        for entry in &schools {
            let (_, school_id) = parse_nsaa_school(entry, &url_of(entry), OBSERVED_ON);
            coaches += nsaa_coaches(entry, &school_id, &url_of(entry), OBSERVED_ON).len();
            for role in &entry.roles {
                if matches!(
                    parse_nsaa_row(&role.label),
                    Some(NsaaRow::SportCoach { .. })
                ) {
                    slots += 1;
                    if !split_person_names(&role.name).is_empty() {
                        named += 1;
                    }
                }
            }
        }
        assert_eq!(
            slots, 30,
            "8 schools × 4 TF/XC sides, minus Anselmo-Merna's two XC sides (it sponsors neither)"
        );
        assert_eq!(
            named, 30,
            "every published TF/XC coach slot is named: 30/30"
        );
        assert_eq!(
            coaches, 42,
            "unique coach entities across the 8 fixture schools"
        );
    }

    #[test]
    fn nsaa_entities_carry_no_emails_anywhere() {
        let schools = parse_nsaa_directory(NSAA_PAGE);
        for entry in &schools {
            let (school, school_id) = parse_nsaa_school(entry, &url_of(entry), OBSERVED_ON);
            assert!(
                !serialized(&school).contains('@'),
                "no email in {}",
                school.name
            );
            for coach in nsaa_coaches(entry, &school_id, &url_of(entry), OBSERVED_ON) {
                assert_eq!(coach.professional_email, None);
                assert_eq!(coach.phone, None);
                assert!(
                    !serialized(&coach).contains('@'),
                    "no email or handle in coach {}",
                    coach.name
                );
                assert_eq!(coach.evidence.len(), 1);
                assert_eq!(coach.evidence[0].observed_on, OBSERVED_ON);
                assert_eq!(coach.evidence[0].source.id, "nsaa");
            }
        }
    }

    #[test]
    fn nd_entities_carry_no_emails_anywhere() {
        let (school, school_id) =
            parse_nd_school_page(ND_PAGE, &sheyenne(), OBSERVED_ON).expect("school");
        assert!(!serialized(&school).contains('@'));
        let mut produced = nd_ad_coaches(&parse_nd_staff(ND_PAGE), &school_id, "u", OBSERVED_ON);
        produced.extend(nd_sport_coaches(
            &parse_nd_offerings(ND_PAGE),
            &school_id,
            "u",
            OBSERVED_ON,
        ));
        assert!(!produced.is_empty());
        for coach in &produced {
            assert_eq!(coach.professional_email, None);
            assert_eq!(coach.phone, None);
            assert!(
                !serialized(coach).contains('@'),
                "no email in coach {}",
                coach.name
            );
        }
        // The fixture page itself has no email at all — the provider publishes no email layer.
        assert!(!EMAIL_REGEX.is_match(ND_PAGE));
    }

    #[test]
    fn nsaa_malformed_input_yields_no_rows() {
        assert!(parse_nsaa_directory("<!doctype html><html>nothing</html>").is_empty());
        assert!(parse_nsaa_directory("").is_empty());
        // A heading without a closing tag, and a heading with an empty name, are both skipped.
        assert!(parse_nsaa_directory(r#"<h1 class="mt-3">Broken"#).is_empty());
        assert!(parse_nsaa_directory(r#"<h1 class="mt-3">   </h1><table></table>"#).is_empty());
        assert_eq!(parse_nsaa_row(""), None);
        let school = NsaaSchool {
            name: String::new(),
            city: None,
            enrollment: None,
            homepage: None,
            roles: Vec::new(),
        };
        let (_, school_id) = parse_nsaa_school(&school, &url_of(&school), OBSERVED_ON);
        assert!(nsaa_coaches(&school, &school_id, &url_of(&school), OBSERVED_ON).is_empty());
    }

    #[tokio::test]
    async fn collect_skips_providers_it_was_not_asked_for() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = crate::store::Store::open(dir.path().join("store")).expect("store");
        let fetcher = crate::net::Fetcher::new(
            dir.path().join("http"),
            None,
            std::time::Duration::from_millis(1),
            std::collections::HashMap::new(),
        )
        .expect("fetcher");
        let ctx = AdapterContext {
            fetcher: &fetcher,
            store: &store,
            refresh: false,
            school_year: crate::model::SchoolYear(2026),
            observed_on: OBSERVED_ON.to_string(),
        };

        let options = Options {
            states: vec!["IA".to_string()],
            observed_on: OBSERVED_ON.to_string(),
            ..Options::default()
        };
        let report = collect(&ctx, &options)
            .await
            .expect("collect returns a report");
        assert_eq!(report.rows, 0);
        assert_eq!(report.with_email, 0);
        assert_eq!(
            report.requests, 0,
            "no provider ran, so no request was sent"
        );
        assert!(
            report
                .notes
                .iter()
                .any(|note| note.contains("no provider selected")),
            "the report says which states were asked for: {:?}",
            report.notes
        );
    }
}
