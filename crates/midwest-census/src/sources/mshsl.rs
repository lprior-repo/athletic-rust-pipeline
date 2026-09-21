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
//!   Cloudflare-obfuscated professional email that is decoded locally (XOR with the first byte). The same
//!   block publishes office roles (principal, superintendent, AD administrative assistant, trainer,
//!   advisors, Title IX officer, sports representatives): they are parsed and then *rejected*, so no
//!   non-coaching office ever reaches the store.
//! * `GET /jsonapi/views/teams/list_school?views-argument[]=<schoolId>` with a sparse fieldset — the
//!   school's team nodes (`drupal_internal__nid` + path alias), filtered to the track/XC activities.
//! * `GET /api/coaches/<team nid>` — per-team coach records (`name`, `coach_level`, `field_email`).
//!   Records whose `coach_level` is not an MSHSL level (`Non-MSHSL Coach`, `MSHSL Sub-Coach`) are dropped
//!   and an email is kept only when it shares the school's own domain: those records publish
//!   personal-domain addresses. `field_work_phone` mixes school extensions with personal mobiles, so the
//!   field is not even declared in the deserializer. [report 09 §"Retention contract"]
//!
//! Every emitted entity carries the URL it came from plus `options.observed_on`; missing fields stay
//! empty and are reported as notes rather than invented.
//!
//! Interface contract (fixed by `sources/mod.rs`): implement `collect` plus public `parse_*` helpers
//! covered by fixture-backed unit tests. This file is owned by a single writer; do not edit any other
//! module while implementing it.

use crate::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use crate::net::FetchOptions;
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::{bail, Context, Result};
use regex::Regex;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;
use std::sync::LazyLock;

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

/// Compiled once per process, with no panic path: a malformed pattern yields `None`.
static LIST_ROW: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r##"<a href="/schools/([^"#?]+)" class="school-teaser__title">([^<]*)</a>"##).ok()
});
static LOCALITY: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"<span class="locality">([^<]*)</span>"#).ok());
static PAGE_LINK: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"href=["']\?page=([0-9]+)["']"#).ok());
static PAGER_NEXT: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"<a[^>]*rel="next"[^>]*>"#).ok());
static PAGE_TITLE: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r#"<h1[^>]*class="[^"]*heading--page-title[^"]*"[^>]*>\s*<div>([^<]*)</div>"#).ok()
});
static GROUP_ID: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"/group/([0-9]{1,8})/"#).ok());
static ENROLLMENT: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"MSHSL Classification Enrollment:\s*([0-9,]{1,12})"#).ok());
static WEBSITE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"<a\s+href="([^"]+)"\s+class="school-intro__link""#).ok());
static ADMIN_ROLE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"<strong>([^<]*)</strong>"#).ok());
static ADMIN_NAME: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"</strong>\s*<div>([^<]*)</div>"#).ok());
static ADMIN_NAME_TEXT: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"</strong>([^<]{2,80})"#).ok());
static CF_HREF: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"email-protection#([0-9a-fA-F]+)"#).ok());
static CF_ATTR: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"data-cfemail="([0-9a-fA-F]+)""#).ok());

/// One row of the `/schools` listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchoolListRow {
    /// Page slug (`/schools/<slug>`).
    pub slug: String,
    pub name: String,
    pub city: Option<String>,
}

/// One entry of a school page's Administration block.
///
/// Every address published for the entry is kept (an entry can carry more than one `mailto`); the first
/// decoded address is the one emitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminEntry {
    pub role: String,
    pub name: String,
    pub emails: Vec<String>,
}

impl AdminEntry {
    pub fn email(&self) -> Option<&str> {
        self.emails.first().map(String::as_str)
    }
}

/// A parsed school detail page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchoolDetail {
    /// `<h1>` title, used only when the listing row has no name.
    pub name: Option<String>,
    /// MSHSL numeric school id, read from the page's `/group/<id>/` links.
    pub school_id: Option<String>,
    pub enrollment: Option<u32>,
    pub website: Option<String>,
    pub admin: Vec<AdminEntry>,
}

/// One team node of `/jsonapi/views/teams/list_school`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamNode {
    /// `drupal_internal__nid` — the key `/api/coaches/<nid>` takes.
    pub nid: String,
    pub title: String,
    /// Path alias, e.g. `/schools/wayzata-high-school/track-and-field-boys/2027`.
    pub alias: String,
}

impl TeamNode {
    /// Canonical MSHSL page for the team, when the alias is a path.
    pub fn page_url(&self) -> Option<String> {
        self.alias
            .starts_with('/')
            .then(|| format!("https://www.mshsl.org{}", self.alias))
    }
}

/// One record of `/api/coaches/<nid>`.
///
/// `field_work_phone` (school extensions and personal mobiles) and `title` are deliberately not part of
/// this struct: they are never read.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CoachRecord {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "coach_level")]
    pub level: String,
    #[serde(default, rename = "field_email")]
    pub email: Option<String>,
}

/// A team node plus the coach records fetched for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamCoaches {
    pub node: TeamNode,
    /// `/api/coaches/<nid>` URL the records came from.
    pub api_url: String,
    pub records: Vec<CoachRecord>,
}

#[derive(Debug, Deserialize)]
struct TeamsPayload {
    #[serde(default)]
    data: Vec<TeamNodeRow>,
}

#[derive(Debug, Deserialize)]
struct TeamNodeRow {
    #[serde(default)]
    attributes: TeamNodeAttributes,
}

#[derive(Debug, Default, Deserialize)]
struct TeamNodeAttributes {
    #[serde(default, rename = "drupal_internal__nid")]
    nid: Option<u64>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    path: Option<TeamPathAlias>,
}

#[derive(Debug, Deserialize)]
struct TeamPathAlias {
    #[serde(default)]
    alias: Option<String>,
}

/// Decode a Cloudflare-obfuscated address: the first byte is the XOR key for the rest.
///
/// Returns `None` for anything that is not a decodable hex payload (odd length, non-hex, empty result,
/// non-UTF-8, or longer than [`MAX_CFEMAIL_HEX`]).
pub fn decode_cfemail(encoded: &str) -> Option<String> {
    let hex = encoded.trim();
    if hex.is_empty() || hex.len() > MAX_CFEMAIL_HEX || !hex.len().is_multiple_of(2) {
        return None;
    }
    if !hex.chars().all(|digit| digit.is_ascii_hexdigit()) {
        return None;
    }
    let digits: Vec<char> = hex.chars().collect();
    let mut bytes: Vec<u8> = Vec::with_capacity(digits.len() / 2);
    for pair in digits.chunks_exact(2) {
        let high = pair.first()?.to_digit(16)?;
        let low = pair.get(1)?.to_digit(16)?;
        bytes.push((high * 16 + low) as u8);
    }
    let key = *bytes.first()?;
    let decoded: Vec<u8> = bytes.iter().skip(1).map(|byte| byte ^ key).collect();
    let address = String::from_utf8(decoded).ok()?.trim().to_string();
    (!address.is_empty()).then_some(address)
}

/// Every Cloudflare-obfuscated address inside one HTML fragment, decoded and de-duplicated.
///
/// Both forms are read: the `email-protection#<hex>` href the server renders and the
/// `data-cfemail="<hex>"` attribute a DOM snapshot can carry.
pub fn decode_cfemail_fragment(fragment: &str) -> Vec<String> {
    let mut addresses: Vec<String> = Vec::new();
    for pattern in [CF_HREF.as_ref(), CF_ATTR.as_ref()].into_iter().flatten() {
        for capture in pattern.captures_iter(fragment) {
            let Some(token) = capture.get(1) else {
                continue;
            };
            let Some(address) = decode_cfemail(token.as_str()) else {
                continue;
            };
            if !addresses.contains(&address) {
                addresses.push(address);
            }
        }
    }
    addresses
}

/// Decode the HTML entities MSHSL uses in names.
fn decode_entities(value: &str) -> String {
    value
        .replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&apos;", "'")
        .replace("&quot;", "\"")
        .replace("&nbsp;", " ")
        .replace("&#160;", " ")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        // `&amp;` last so `&amp;lt;` cannot turn into a tag.
        .replace("&amp;", "&")
}

/// Collapse whitespace and decode entities.
fn clean(value: &str) -> String {
    decode_entities(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Strip leading honorifics so "Mr. Barry Mink" and "Barry Mink" mint the same coach identity.
pub fn strip_honorific(value: &str) -> String {
    let mut parts: Vec<&str> = value.split_whitespace().collect();
    while let Some(first) = parts.first() {
        let token = first.trim_end_matches('.').to_ascii_lowercase();
        if matches!(
            token.as_str(),
            "mr" | "mrs" | "ms" | "miss" | "dr" | "coach" | "coach." | "sir" | "rev"
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

/// Normalise a role label: trim, drop a trailing colon, collapse inner whitespace, lowercase.
fn normalize_label(label: &str) -> String {
    label
        .trim()
        .trim_end_matches(':')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

/// Map an Administration-block role onto an athletic-director role.
///
/// Exact match on purpose: the same block publishes office staff whose labels contain
/// "activities director" ("AD Administrative Assistant", "Activities Director Secretary"), and those
/// must never be emitted as the school's activities director.
pub fn ad_role(label: &str) -> Option<CoachRole> {
    match normalize_label(label).as_str() {
        "activities director"
        | "athletic director"
        | "assistant activities director"
        | "assistant athletic director"
        | "associate activities director"
        | "associate athletic director" => Some(CoachRole::AthleticDirector),
        _ => None,
    }
}

/// The school universe as published by the `/schools` listing.
pub fn parse_school_list(html: &str) -> Vec<SchoolListRow> {
    let (Some(rows), Some(locality)) = (LIST_ROW.as_ref(), LOCALITY.as_ref()) else {
        return Vec::new();
    };
    const ROW_MARKER: &str = "<div class=\"views-row\">";
    let chunks: Vec<&str> = if html.contains(ROW_MARKER) {
        html.split(ROW_MARKER).skip(1).collect()
    } else {
        vec![html]
    };
    let mut school_rows: Vec<SchoolListRow> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for chunk in chunks {
        let Some(capture) = rows.captures(chunk) else {
            continue;
        };
        let (Some(slug), Some(name)) = (capture.get(1), capture.get(2)) else {
            continue;
        };
        let slug = slug.as_str().trim().to_string();
        let name = clean(name.as_str());
        if slug.is_empty() || slug.contains('/') || name.is_empty() {
            continue;
        }
        if !seen.insert(slug.clone()) {
            continue;
        }
        let city = locality
            .captures(chunk)
            .and_then(|capture| capture.get(1))
            .map(|value| clean(value.as_str()))
            .filter(|value| !value.is_empty());
        school_rows.push(SchoolListRow { slug, name, city });
    }
    school_rows
}

/// Next listing page number, from the pager markup.
///
/// A page is followed when the pager links to `current + 1`: either as a numbered page link or through
/// the "next" anchor, which is what a Drupal pager renders on every page but the last.
pub fn parse_next_listing_page(html: &str, current: usize) -> Option<usize> {
    let next = current.checked_add(1)?;
    let link = PAGE_LINK.as_ref()?;
    let numbered = link
        .captures_iter(html)
        .filter_map(|capture| capture.get(1))
        .filter_map(|value| value.as_str().parse::<usize>().ok())
        .any(|page| page == next);
    if numbered {
        return Some(next);
    }
    let anchor = PAGER_NEXT.as_ref()?;
    let followed = anchor.find_iter(html).any(|tag| {
        link.captures(tag.as_str())
            .and_then(|capture| capture.get(1))
            .and_then(|value| value.as_str().parse::<usize>().ok())
            .is_some_and(|page| page == next)
    });
    followed.then_some(next)
}

/// URL of a `/schools` listing page.
pub fn listing_page_url(page: usize) -> String {
    if page == 0 {
        SCHOOL_LIST_URL.to_string()
    } else {
        format!("{SCHOOL_LIST_URL}?page={page}")
    }
}

/// URL of a school page.
pub fn school_page_url(slug: &str) -> String {
    format!("{SCHOOL_URL_PREFIX}{slug}")
}

/// The `Administration` grid of a school page, up to the next section heading.
fn administration_block(html: &str) -> Option<&str> {
    let start = html.find("grid--administration")?;
    let tail = html.get(start..)?;
    let end = ["<strong>Conference", "<h2"]
        .iter()
        .filter_map(|marker| tail.find(marker))
        .min()
        .unwrap_or(tail.len());
    tail.get(..end)
}

/// The Administration block as role/name/address entries, in document order.
pub fn parse_admin_entries(html: &str) -> Vec<AdminEntry> {
    let Some(block) = administration_block(html) else {
        return Vec::new();
    };
    let (Some(role_re), Some(name_re), Some(text_re)) = (
        ADMIN_ROLE.as_ref(),
        ADMIN_NAME.as_ref(),
        ADMIN_NAME_TEXT.as_ref(),
    ) else {
        return Vec::new();
    };
    const ITEM_MARKER: &str = "<div class=\"grid__item\">";
    let mut entries: Vec<AdminEntry> = Vec::new();
    for item in block.split(ITEM_MARKER).skip(1) {
        let Some(role) = role_re
            .captures(item)
            .and_then(|capture| capture.get(1))
            .map(|value| clean(value.as_str()).trim_end_matches(':').to_string())
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let name = name_re
            .captures(item)
            .and_then(|capture| capture.get(1))
            .map(|value| clean(value.as_str()))
            .or_else(|| {
                text_re
                    .captures(item)
                    .and_then(|capture| capture.get(1))
                    .map(|value| clean(value.as_str()))
            })
            .unwrap_or_default();
        entries.push(AdminEntry {
            role,
            name,
            emails: decode_cfemail_fragment(item),
        });
    }
    entries
}

/// Parse a school page: identity, facts and Administration block.
pub fn parse_school_detail(html: &str) -> SchoolDetail {
    let name = PAGE_TITLE
        .as_ref()
        .and_then(|pattern| pattern.captures(html))
        .and_then(|capture| capture.get(1))
        .map(|value| clean(value.as_str()))
        .filter(|value| !value.is_empty());
    let school_id = GROUP_ID
        .as_ref()
        .and_then(|pattern| pattern.captures(html))
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str().to_string());
    let enrollment = ENROLLMENT
        .as_ref()
        .and_then(|pattern| pattern.captures(html))
        .and_then(|capture| capture.get(1))
        .and_then(|value| value.as_str().replace(',', "").parse::<u32>().ok())
        .filter(|value| *value > 0 && *value < 100_000);
    let website = WEBSITE
        .as_ref()
        .and_then(|pattern| pattern.captures(html))
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str().trim().to_string())
        .filter(|value| value.starts_with("http://") || value.starts_with("https://"));
    SchoolDetail {
        name,
        school_id,
        enrollment,
        website,
        admin: parse_admin_entries(html),
    }
}

/// The provider's own key for a school: its numeric id when published, else the page slug.
pub fn provider_key(row: &SchoolListRow, detail: &SchoolDetail) -> String {
    detail.school_id.clone().unwrap_or_else(|| row.slug.clone())
}

/// Build the canonical school for one listing row plus its detail page.
///
/// The id is minted from state + normalized name, so the same school seen by another adapter resolves to
/// the same canonical id.
pub fn school_entities(
    row: &SchoolListRow,
    detail: &SchoolDetail,
    page_url: &str,
    observed_on: &str,
) -> Option<(CanonicalSchool, SchoolId)> {
    let name = if row.name.trim().is_empty() {
        detail.name.clone()?
    } else {
        row.name.clone()
    };
    let (mut school, school_id) = CanonicalSchool::new("MN", &name, normalize_name(&name));
    school.city = row.city.clone();
    school.association = Some(SOURCE_ID.to_string());
    school.enrollment = detail.enrollment;
    school.school_website = detail.website.clone();
    school.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: SOURCE_ID.to_string(),
            },
            provider_key(row, detail),
        )
        .with_url(page_url.to_string()),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(page_url.to_string())),
        observed_on,
    ));
    Some((school, school_id))
}

/// Canonical athletic-director rows for one school page.
///
/// Only `Activities Director` / `Assistant Activities Director` entries are emitted; office roles are
/// dropped here, which is the last point before the store.
pub fn ad_coaches(
    detail: &SchoolDetail,
    school_id: &SchoolId,
    school_key: &str,
    page_url: &str,
    observed_on: &str,
) -> Vec<CanonicalCoach> {
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for entry in &detail.admin {
        if ad_role(&entry.role).is_none() {
            continue;
        }
        let name = strip_honorific(&entry.name);
        if name.is_empty() {
            continue;
        }
        let mut coach = CanonicalCoach::new(
            school_id,
            name.as_str(),
            None,
            Gender::Mixed,
            CoachRole::AthleticDirector,
        );
        coach.professional_email = entry.email().map(str::to_string);
        coach.source_identities.push(
            SourceIdentity::new(
                SourceNamespace::AssociationSchool {
                    association: SOURCE_ID.to_string(),
                },
                format!("{school_key}:ad"),
            )
            .with_url(page_url.to_string()),
        );
        coach.evidence.push(Evidence::parsed(
            SourceRef::new(SOURCE_ID, Some(page_url.to_string())),
            observed_on,
        ));
        if seen.insert(coach.id.as_str().to_string()) {
            coaches.push(coach);
        }
    }
    coaches
}

/// The domains this school's own contacts are published on: its AD addresses plus its website host.
pub fn school_domains(detail: &SchoolDetail) -> Vec<String> {
    let mut domains: Vec<String> = Vec::new();
    for entry in &detail.admin {
        if ad_role(&entry.role).is_none() {
            continue;
        }
        for address in &entry.emails {
            if let Some(domain) = email_domain(address) {
                push_unique(&mut domains, domain);
            }
        }
    }
    if let Some(domain) = detail.website.as_deref().and_then(host_domain) {
        push_unique(&mut domains, domain);
    }
    domains
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn email_domain(address: &str) -> Option<String> {
    let (_, domain) = address.trim().rsplit_once('@')?;
    let domain = domain.trim().trim_matches('.').to_ascii_lowercase();
    (!domain.is_empty() && domain.contains('.')).then_some(domain)
}

fn host_domain(url: &str) -> Option<String> {
    let authority = url.split("://").nth(1).unwrap_or(url);
    let host = authority
        .split('/')
        .next()?
        .rsplit('@')
        .next()?
        .split(':')
        .next()?
        .trim_start_matches("www.")
        .to_ascii_lowercase();
    (!host.is_empty()).then_some(host)
}

/// True when `domain` is `base` or a subdomain of it.
fn domain_within(domain: &str, base: &str) -> bool {
    domain == base
        || domain
            .strip_suffix(base)
            .is_some_and(|prefix| prefix.ends_with('.'))
}

/// Keep an API-published coach address only when it is not a consumer mailbox and shares the school's
/// own domain.
pub fn accept_coach_email(address: &str, domains: &[String]) -> Option<String> {
    let address = address.trim();
    let domain = email_domain(address)?;
    if crate::model::CONSUMER_MAIL_DOMAINS
        .iter()
        .any(|consumer| domain_within(&domain, consumer))
    {
        return None;
    }
    domains
        .iter()
        .any(|known| domain_within(&domain, known))
        .then(|| address.to_string())
}

/// Map a team path alias onto sport and gender.
///
/// Aliases are `/schools/<slug>/<activity>/<year>`; `<activity>` is the provider's own vocabulary
/// (`cross-country-running-boys`, `track-and-field-girls`).
pub fn team_sport(alias: &str) -> Option<(Sport, Gender)> {
    let lowered = alias.to_ascii_lowercase();
    let activity = lowered.split('/').filter(|part| !part.is_empty()).nth(2)?;
    let sport = if activity.contains("cross-country") {
        Sport::CrossCountry
    } else if activity.contains("indoor-track") {
        Sport::IndoorTrack
    } else if activity.contains("track-and-field") || activity.contains("track-field") {
        Sport::OutdoorTrack
    } else {
        return None;
    };
    let gender = if activity.contains("boys") {
        Gender::Boys
    } else if activity.contains("girls") {
        Gender::Girls
    } else {
        Gender::Mixed
    };
    Some((sport, gender))
}

/// The team nodes of a school's track/XC teams, one per sport+gender.
pub fn select_team_nodes(nodes: &[TeamNode]) -> Vec<TeamNode> {
    let mut selected: Vec<TeamNode> = Vec::new();
    let mut seen: Vec<(Sport, Gender)> = Vec::new();
    for node in nodes {
        if selected.len() >= MAX_TEAMS_PER_SCHOOL {
            break;
        }
        let Some(key) = team_sport(&node.alias) else {
            continue;
        };
        if !seen.contains(&key) {
            seen.push(key);
            selected.push(node.clone());
        }
    }
    selected
}

/// Parse `/jsonapi/views/teams/list_school`; rows without a nid are dropped.
pub fn parse_team_nodes(payload: &str) -> Vec<TeamNode> {
    let Ok(parsed) = serde_json::from_str::<TeamsPayload>(payload) else {
        return Vec::new();
    };
    parsed
        .data
        .into_iter()
        .filter_map(|row| {
            let nid = row.attributes.nid?;
            Some(TeamNode {
                nid: nid.to_string(),
                title: row.attributes.title.unwrap_or_default().trim().to_string(),
                alias: row
                    .attributes
                    .path
                    .and_then(|path| path.alias)
                    .unwrap_or_default(),
            })
        })
        .collect()
}

/// Parse `/api/coaches/<nid>`; a payload that is not an array of records yields no rows.
pub fn parse_coach_records(payload: &str) -> Vec<CoachRecord> {
    let Ok(rows) = serde_json::from_str::<Vec<CoachRecord>>(payload) else {
        return Vec::new();
    };
    rows.into_iter()
        .map(|mut row| {
            row.name = clean(&row.name);
            row.level = row.level.trim().to_string();
            row.email = row
                .email
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty());
            row
        })
        .collect()
}

/// The site's own renderer drops these levels: they carry personal-domain addresses and personal
/// mobiles, so they are not MSHSL coaching staff. [report 09, `teampersonnel.bundle.js`]
pub fn is_published_level(level: &str) -> bool {
    !matches!(
        level.trim().to_ascii_lowercase().as_str(),
        "non-mshsl coach" | "mshsl sub-coach"
    )
}

/// Map an MSHSL coach level onto our role vocabulary; unknown levels are skipped.
pub fn coach_role(level: &str) -> Option<CoachRole> {
    match level.trim().to_ascii_lowercase().as_str() {
        "head coach" => Some(CoachRole::HeadCoach),
        "assistant coach" => Some(CoachRole::AssistantCoach),
        _ => None,
    }
}

/// Canonical coach entities for the team coach payloads fetched for one school.
pub fn coach_entities(
    teams: &[TeamCoaches],
    school_id: &SchoolId,
    domains: &[String],
    observed_on: &str,
) -> Vec<CanonicalCoach> {
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for team in teams {
        let Some((sport, gender)) = team_sport(&team.node.alias) else {
            continue;
        };
        for record in &team.records {
            if !is_published_level(&record.level) {
                continue;
            }
            let Some(role) = coach_role(&record.level) else {
                continue;
            };
            let name = strip_honorific(&record.name);
            if name.is_empty() {
                continue;
            }
            let mut coach =
                CanonicalCoach::new(school_id, name.as_str(), Some(sport), gender, role);
            coach.professional_email = record
                .email
                .as_deref()
                .and_then(|address| accept_coach_email(address, domains));
            coach.source_identities.push(
                SourceIdentity::new(
                    SourceNamespace::Other(COACH_NAMESPACE.to_string()),
                    format!("{}:{}", team.node.nid, normalize_name(&name)),
                )
                .with_url(team.api_url.clone()),
            );
            coach.evidence.push(Evidence::parsed(
                SourceRef::new(SOURCE_ID, Some(team.api_url.clone())),
                observed_on,
            ));
            if let Some(page_url) = team.node.page_url() {
                coach.evidence.push(Evidence::parsed(
                    SourceRef::new(SOURCE_ID, Some(page_url)),
                    observed_on,
                ));
            }
            if seen.insert(coach.id.as_str().to_string()) {
                coaches.push(coach);
            }
        }
    }
    coaches
}

/// Fetch one school's team nodes and their coach records.
///
/// Returns the entities plus notes for surfaces that were unavailable; the caller has already written the
/// AD rows, so a failure here never discards work.
async fn collect_team_coaches(
    ctx: &AdapterContext<'_>,
    fetch_options: &FetchOptions,
    school_key: &str,
    school_id: &SchoolId,
    domains: &[String],
    observed_on: &str,
) -> (Vec<CanonicalCoach>, Vec<String>) {
    let teams_url = format!(
        "{TEAMS_VIEW_URL}?views-argument%5B%5D={school_key}&fields%5Bnode--participant%5D=title,path,drupal_internal__nid"
    );
    let outcome = match ctx.fetcher.get(&teams_url, fetch_options).await {
        Ok(outcome) => outcome,
        Err(error) => {
            return (
                Vec::new(),
                vec![format!("team list {teams_url}: {error:#}")],
            )
        }
    };
    let selected = select_team_nodes(&parse_team_nodes(&outcome.text()));
    let mut teams: Vec<TeamCoaches> = Vec::with_capacity(selected.len());
    let mut notes: Vec<String> = Vec::new();
    for node in selected {
        let api_url = format!("{COACH_API_PREFIX}{}", node.nid);
        match ctx.fetcher.get(&api_url, fetch_options).await {
            Ok(outcome) => teams.push(TeamCoaches {
                node,
                api_url,
                records: parse_coach_records(&outcome.text()),
            }),
            Err(error) => notes.push(format!("coach list {api_url}: {error:#}")),
        }
    }
    (
        coach_entities(&teams, school_id, domains, observed_on),
        notes,
    )
}

/// Fetch options for this run: the run-level refresh flag or the adapter's own.
fn fetch_options(ctx: &AdapterContext<'_>, options: &Options) -> FetchOptions {
    FetchOptions {
        refresh: options.refresh || ctx.refresh,
        allow_not_found: false,
        headers: Vec::new(),
    }
}

/// Collect this provider's schools and coach/AD contacts into the canonical store.
///
/// Per school: one school page (facts + AD rows), one team-node list and up to
/// [`MAX_TEAMS_PER_SCHOOL`] coach requests. Progress is journalled per school, so a re-run resumes
/// without re-fetching finished schools.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let mut report = AdapterReport::new(SOURCE_ID, "schools");
    let stats_before = ctx.fetcher.stats().await;
    if !options.states.is_empty()
        && !options
            .states
            .iter()
            .any(|state| state.trim().eq_ignore_ascii_case("MN"))
    {
        report.note("MSHSL covers Minnesota only; requested states do not include MN, so nothing was fetched");
        return Ok(report);
    }
    let wanted: HashSet<String> = options
        .school_names
        .iter()
        .map(|name| normalize_name(name))
        .filter(|name| !name.is_empty())
        .collect();
    let done = ctx.store.journal_keys("mshsl_schools")?;
    let fetch = fetch_options(ctx, options);
    let mut processed: u64 = 0;
    let mut skipped: u64 = 0;
    let mut unparsed: u64 = 0;
    let mut ad_rows: u64 = 0;
    let mut coach_rows: u64 = 0;
    let mut with_email: u64 = 0;
    let mut office_roles: u64 = 0;
    let mut page = 0usize;
    'pages: while page < MAX_LISTING_PAGES {
        let url = listing_page_url(page);
        let outcome = ctx
            .fetcher
            .get(&url, &fetch)
            .await
            .with_context(|| format!("fetching MSHSL school listing page {page}"))?;
        let html = outcome.text();
        let rows = parse_school_list(&html);
        if rows.is_empty() {
            if page == 0 {
                bail!("MSHSL school listing {url} contained no school rows (markup change or empty page)");
            }
            report.note(format!(
                "listing page {url} contained no school rows; pagination stopped"
            ));
            break;
        }
        for row in rows {
            if options.limit.is_some_and(|limit| processed >= limit as u64) {
                break 'pages;
            }
            if !wanted.is_empty() && !wanted.contains(&normalize_name(&row.name)) {
                continue;
            }
            let key = format!("MN:{}", row.slug);
            if done.contains(&key) {
                skipped += 1;
                continue;
            }
            let page_url = school_page_url(&row.slug);
            let detail = match ctx.fetcher.get(&page_url, &fetch).await {
                Ok(outcome) => parse_school_detail(&outcome.text()),
                Err(error) => {
                    report.note(format!("school page {page_url}: {error:#}"));
                    continue;
                }
            };
            let Some((school, school_id)) =
                school_entities(&row, &detail, &page_url, &options.observed_on)
            else {
                unparsed += 1;
                report.note(format!(
                    "school page {page_url}: no school name in listing row or page"
                ));
                continue;
            };
            let school_key = provider_key(&row, &detail);
            let ads = ad_coaches(
                &detail,
                &school_id,
                &school_key,
                &page_url,
                &options.observed_on,
            );
            let domains = school_domains(&detail);
            office_roles += detail
                .admin
                .iter()
                .filter(|entry| ad_role(&entry.role).is_none())
                .count() as u64;
            ctx.store.append(Table::Schools, &school)?;
            ctx.store.append_many(Table::Coaches, &ads)?;
            let (sport_coaches, notes) = match detail.school_id.as_deref() {
                Some(id) => {
                    collect_team_coaches(
                        ctx,
                        &fetch,
                        id,
                        &school_id,
                        &domains,
                        &options.observed_on,
                    )
                    .await
                }
                None => (
                    Vec::new(),
                    vec![format!(
                        "school page {page_url}: no /group/<id>/ link, sport coaches skipped"
                    )],
                ),
            };
            ctx.store.append_many(Table::Coaches, &sport_coaches)?;
            for note in notes {
                report.note(format!("{}: {note}", school.name));
            }
            let school_with_email = ads
                .iter()
                .chain(sport_coaches.iter())
                .filter(|coach| coach.professional_email.is_some())
                .count() as u64;
            ad_rows += ads.len() as u64;
            coach_rows += sport_coaches.len() as u64;
            with_email += school_with_email;
            ctx.store.journal_done(
                "mshsl_schools",
                &key,
                &json!({
                    "school": school.name,
                    "school_id": school_key,
                    "page_url": page_url,
                    "city": row.city,
                    "ad_rows": ads.len(),
                    "sport_coach_rows": sport_coaches.len(),
                    "with_email": school_with_email,
                    "observed_on": options.observed_on,
                }),
            )?;
            ctx.store.journal_done(
                "mshsl_coaches",
                &key,
                &json!({
                    "school_id": school_key,
                    "ad_rows": ads.len(),
                    "sport_coach_rows": sport_coaches.len(),
                    "with_email": school_with_email,
                    "observed_on": options.observed_on,
                }),
            )?;
            processed += 1;
        }
        match parse_next_listing_page(&html, page) {
            Some(next) => page = next,
            None => break,
        }
    }
    let stats_after = ctx.fetcher.stats().await;
    report.rows = processed;
    report.requests = stats_after.requests.saturating_sub(stats_before.requests);
    report.from_cache = stats_after
        .cache_hits
        .saturating_sub(stats_before.cache_hits);
    report.errors = stats_after.errors.saturating_sub(stats_before.errors) + unparsed;
    report.with_email = with_email;
    report.note(format!(
        "{processed} school(s) processed ({skipped} already journalled): {ad_rows} athletic-director row(s), {coach_rows} sport-coach row(s), {with_email} with a professional email"
    ));
    report.note(format!(
        "{office_roles} Administration-block entry/entries were office roles (principal, superintendent, AD administrative assistant, trainer, advisors, Title IX, sports representatives) and were not emitted"
    ));
    report.note(
        "AD contacts come from the school page Administration block (Cloudflare-obfuscated addresses decoded locally); sport coaches come from /api/coaches/<team nid> reached through /jsonapi/views/teams/list_school, filtered to MSHSL coach levels and school-domain addresses - personal-domain addresses and every phone column are never parsed",
    );
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LISTING: &str = include_str!("../../tests/fixtures/mshsl/schools_listing.html");
    const AITKIN: &str =
        include_str!("../../tests/fixtures/mshsl/school_detail_aitkin-high-school.html");
    const FOLEY: &str =
        include_str!("../../tests/fixtures/mshsl/school_detail_foley-high-school.html");
    const WAYZATA: &str =
        include_str!("../../tests/fixtures/mshsl/school_detail_wayzata-high-school.html");
    const ACADEMIC_ARTS: &str =
        include_str!("../../tests/fixtures/mshsl/school_detail_academic-arts-high-school.html");
    // Live captures (2026-09-20, HTTP 200) of
    // https://www.mshsl.org/jsonapi/views/teams/list_school?views-argument[]=611 (sparse fieldset) and
    // https://www.mshsl.org/api/coaches/589034, verbatim.
    const WAYZATA_TEAMS: &str = include_str!("../../tests/fixtures/mshsl/team_nodes_wayzata.json");
    const WAYZATA_TF_COACHES: &str =
        include_str!("../../tests/fixtures/mshsl/coach_records_wayzata_track_boys.json");
    // Live captures (2026-09-20T14:39Z, HTTP 200, verbatim) of school 7's team list and of the two
    // coach payloads its two track nodes point at, plus the page-0 listing with the pager trimmed so
    // the cached run below stays on one page.
    const LISTING_FIRST_PAGE: &str =
        include_str!("../../tests/fixtures/mshsl/schools_listing_first_page.html");
    const AITKIN_TEAMS: &str = include_str!("../../tests/fixtures/mshsl/team_nodes_aitkin.json");
    const AITKIN_TF_BOYS: &str =
        include_str!("../../tests/fixtures/mshsl/coach_records_aitkin_track-and-field-boys.json");
    const AITKIN_TF_GIRLS: &str =
        include_str!("../../tests/fixtures/mshsl/coach_records_aitkin_track-and-field-girls.json");

    const OBSERVED_ON: &str = "2026-09-20";

    /// Seed the fetcher's on-disk cache for `url` under the key `Fetcher` derives
    /// (`sha256(method \x1f url \x1f body)[..16]`), so `collect` can be driven end to end with no socket.
    fn seed_cache(cache_dir: &std::path::Path, url: &str, body: &str) {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"GET");
        hasher.update([0x1f]);
        hasher.update(url.as_bytes());
        hasher.update([0x1f]);
        let key: String = hasher.finalize()[..16]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        let meta = json!({
            "url": url,
            "method": "GET",
            "status": 200,
            "sha256": format!("{:x}", Sha256::digest(body.as_bytes())),
            "bytes": body.len(),
            "fetched_at": "2026-09-20T14:39:00Z",
        });
        std::fs::create_dir_all(cache_dir).expect("cache dir");
        std::fs::write(cache_dir.join(format!("{key}.body")), body).expect("cache body");
        std::fs::write(
            cache_dir.join(format!("{key}.meta.json")),
            serde_json::to_vec(&meta).expect("cache meta"),
        )
        .expect("cache meta written");
    }

    fn listing_row<'a>(rows: &'a [SchoolListRow], slug: &str) -> &'a SchoolListRow {
        rows.iter()
            .find(|row| row.slug == slug)
            .expect("fixture row")
    }

    /// A listing row for a school whose listing page was not captured: the school's own page supplies the
    /// name and the slug is the real one (page 0 of the capture stops at Aitkin, so later schools are
    /// reached on their own pages).
    fn detail_row(slug: &str, detail: &SchoolDetail) -> SchoolListRow {
        SchoolListRow {
            slug: slug.to_string(),
            name: detail.name.clone().unwrap_or_default(),
            city: None,
        }
    }

    /// The listing row the crawl would have seen: the captured one when page 0 holds the school, else a
    /// row built from the school's own page.
    fn row_for(rows: &[SchoolListRow], slug: &str, detail: &SchoolDetail) -> SchoolListRow {
        rows.iter()
            .find(|row| row.slug == slug)
            .cloned()
            .unwrap_or_else(|| detail_row(slug, detail))
    }

    #[test]
    fn listing_rows_carry_slug_name_and_city() {
        let rows = parse_school_list(LISTING);
        assert_eq!(rows.len(), 8, "listing fixture holds eight school rows");
        let first = &rows[0];
        assert_eq!(first.slug, "aasen-home-school");
        assert_eq!(first.name, "Aasen Home School");
        assert_eq!(first.city.as_deref(), Some("Clearwater"));
        let aitkin = listing_row(&rows, "aitkin-high-school");
        assert_eq!(aitkin.name, "Aitkin High School");
        assert_eq!(aitkin.city.as_deref(), Some("Aitkin"));
        assert!(rows
            .iter()
            .all(|row| !row.slug.contains('/') && !row.name.is_empty()));
    }

    #[test]
    fn pagination_follows_the_real_pager() {
        // The capture is listing page 0 and shows numbered links 0..8 plus a "next page" anchor.
        assert_eq!(parse_next_listing_page(LISTING, 0), Some(1));
        assert_eq!(parse_next_listing_page(LISTING, 1), Some(2));
        assert_eq!(
            parse_next_listing_page(LISTING, 8),
            None,
            "the capture has no link to page 9"
        );
        assert_eq!(parse_next_listing_page(LISTING, 13), None);
        assert_eq!(
            parse_next_listing_page(FOLEY, 0),
            None,
            "a detail page has no pager"
        );
        // A pager whose only continuation is the "next" anchor (attributes reordered) is still followed.
        let next_only = r#"<nav class="pager"><a rel="next" class="pager__link" href="?page=9">Next ›</a></nav>"#;
        assert_eq!(parse_next_listing_page(next_only, 8), Some(9));
        assert_eq!(parse_next_listing_page(next_only, 7), None);
        assert_eq!(parse_next_listing_page("", 0), None);
        assert_eq!(listing_page_url(0), "https://www.mshsl.org/schools");
        assert_eq!(listing_page_url(3), "https://www.mshsl.org/schools?page=3");
        assert_eq!(
            school_page_url("foley-high-school"),
            "https://www.mshsl.org/schools/foley-high-school"
        );
    }

    #[test]
    fn cfemail_decodes_to_the_published_addresses() {
        // Real Cloudflare payloads from the three captures.
        assert_eq!(
            decode_cfemail("8de0eff8e8eee6e8fffecdecfdfdfea3e4fee9b8bca3e2ffea").as_deref(),
            Some("mbueckers@apps.isd51.org")
        );
        assert_eq!(
            decode_cfemail("6e04060b001c070d051d01002e071d0a5f40011c09").as_deref(),
            Some("jhenrickson@isd1.org")
        );
        assert_eq!(
            decode_cfemail("254840424d444b0b554a515140576552445c5f44514456464d4a4a49560b4a5742")
                .as_deref(),
            Some("meghan.potter@wayzataschools.org")
        );
        // The attribute form a DOM snapshot carries decodes with the same routine.
        let attributes = decode_cfemail_fragment(
            r#"<span data-cfemail="8de0eff8e8eee6e8fffecdecfdfdfea3e4fee9b8bca3e2ffea"></span>"#,
        );
        assert_eq!(attributes, vec!["mbueckers@apps.isd51.org".to_string()]);
        for malformed in ["", "abc", "zz00", "00", "no-hex-here", "  "] {
            assert_eq!(
                decode_cfemail(malformed),
                None,
                "{malformed:?} must not decode"
            );
        }
        assert!(decode_cfemail_fragment("<p>no addresses here</p>").is_empty());
    }

    #[test]
    fn school_page_yields_canonical_school_with_identity_and_evidence() {
        let rows = parse_school_list(LISTING);
        let detail = parse_school_detail(AITKIN);
        assert_eq!(detail.name.as_deref(), Some("Aitkin High School"));
        assert_eq!(detail.school_id.as_deref(), Some("7"));
        assert_eq!(detail.enrollment, Some(291));
        assert_eq!(
            detail.website.as_deref(),
            Some("https://isd1.rschoolteams.com/")
        );
        let row = listing_row(&rows, "aitkin-high-school");
        let url = school_page_url(&row.slug);
        let (school, school_id) = school_entities(row, &detail, &url, OBSERVED_ON).expect("school");
        assert_eq!(school.name, "Aitkin High School");
        assert_eq!(school.state.as_deref(), Some("MN"));
        assert_eq!(school.city.as_deref(), Some("Aitkin"));
        assert_eq!(school.association.as_deref(), Some("mshsl"));
        assert_eq!(school.enrollment, Some(291));
        assert_eq!(
            school.school_website.as_deref(),
            Some("https://isd1.rschoolteams.com/")
        );
        assert_eq!(
            school.id,
            CanonicalSchool::mint(
                "MN",
                "Aitkin High School",
                &normalize_name("Aitkin High School")
            )
        );
        assert_eq!(school_id, school.id);
        let identity = school.source_identities.first().expect("identity");
        assert_eq!(
            identity.namespace,
            SourceNamespace::AssociationSchool {
                association: "mshsl".to_string()
            }
        );
        assert_eq!(identity.id, "7");
        assert_eq!(identity.url.as_deref(), Some(url.as_str()));
        let evidence = school.evidence.first().expect("evidence");
        assert_eq!(evidence.source.url.as_deref(), Some(url.as_str()));
        assert_eq!(evidence.observed_on, OBSERVED_ON);
        // Neither the listing row nor the page contributes a name: no school can be minted.
        let nameless = SchoolListRow {
            slug: "nameless".to_string(),
            name: String::new(),
            city: None,
        };
        assert!(school_entities(&nameless, &SchoolDetail::default(), &url, OBSERVED_ON).is_none());
        // A listing row name is enough: the school is minted from the row even with no page facts.
        let (from_row, _) =
            school_entities(row, &SchoolDetail::default(), &url, OBSERVED_ON).expect("school");
        assert_eq!(from_row.name, "Aitkin High School");
        assert!(from_row.enrollment.is_none());
    }

    #[test]
    fn athletic_directors_are_the_only_admin_rows_emitted() {
        let cases = [
            (
                AITKIN,
                "aitkin-high-school",
                vec!["jhenrickson@isd1.org", "ahills@isd1.org"],
            ),
            (
                FOLEY,
                "foley-high-school",
                vec!["mbueckers@apps.isd51.org", ""],
            ),
            (
                ACADEMIC_ARTS,
                "academic-arts-high-school",
                vec!["hannah.couch@academicarts.org"],
            ),
        ];
        let rows = parse_school_list(LISTING);
        for (html, slug, emails) in cases {
            let detail = parse_school_detail(html);
            let row = row_for(&rows, slug, &detail);
            let (_, school_id) =
                school_entities(&row, &detail, &school_page_url(slug), OBSERVED_ON)
                    .expect("school");
            let coaches = ad_coaches(
                &detail,
                &school_id,
                &provider_key(&row, &detail),
                &school_page_url(slug),
                OBSERVED_ON,
            );
            assert_eq!(coaches.len(), emails.len(), "{slug}: one row per AD entry");
            for (coach, email) in coaches.iter().zip(emails) {
                assert_eq!(coach.role, CoachRole::AthleticDirector, "{slug}");
                assert_eq!(coach.sport, None, "athletic directors are school-wide");
                assert_eq!(coach.gender, Gender::Mixed);
                assert_eq!(
                    coach.professional_email.as_deref().unwrap_or(""),
                    email,
                    "{slug}"
                );
                assert_eq!(coach.evidence.len(), 1);
                assert_eq!(coach.source_identities.len(), 1);
            }
        }
        // Foley's assistant director is published without a contact: that row exists with no email.
        let foley = parse_school_detail(FOLEY);
        let assistant = foley
            .admin
            .iter()
            .find(|entry| entry.role.contains("Assistant"))
            .expect("assistant director entry");
        assert_eq!(assistant.name, "Alyssa Stewart");
        assert!(assistant.email().is_none());
    }

    #[test]
    fn office_roles_never_become_coaches_or_athletic_directors() {
        // Wayzata publishes fifteen Administration entries, two of them "AD Administrative Assistant"
        // with addresses and an athletic trainer on a hospital domain.
        let detail = parse_school_detail(WAYZATA);
        assert_eq!(detail.admin.len(), 15);
        let foley = parse_school_detail(FOLEY);
        assert!(foley.admin.iter().any(
            |entry| entry.role.contains("Administrative Assistant") && entry.email().is_some()
        ));

        let rows = parse_school_list(LISTING);
        let row = row_for(&rows, "wayzata-high-school", &detail);
        let (school, school_id) =
            school_entities(&row, &detail, &school_page_url(&row.slug), OBSERVED_ON)
                .expect("school");
        let coaches = ad_coaches(
            &detail,
            &school_id,
            "611",
            &school_page_url(&row.slug),
            OBSERVED_ON,
        );
        assert_eq!(coaches.len(), 2, "only the AD and the assistant AD");
        let emitted = serde_json::to_string(&(school, coaches)).expect("json");
        for office in [
            "chris.easton@wayzataschools.org",
            "kari.rohrich@wayzataschools.org",
            "chris.thein@wayzataschools.org",
            "Chris Easton",
            "Kari Rohrich",
            "Scott Gengler",
            "Robb Virgin",
            "Donald Krubsack",
        ] {
            assert!(!emitted.contains(office), "{office} must not be emitted");
        }
        assert!(emitted.contains("meghan.potter@wayzataschools.org"));
        assert!(emitted.contains("sydney.helmbrecht@wayzataschools.org"));
        // Foley publishes its AD administrative assistant with an address in the same block.
        let foley = parse_school_detail(FOLEY);
        let foley_row = row_for(&rows, "foley-high-school", &foley);
        let foley_url = school_page_url(&foley_row.slug);
        let (foley_school, foley_id) =
            school_entities(&foley_row, &foley, &foley_url, OBSERVED_ON).expect("school");
        let foley_emitted = serde_json::to_string(&(
            foley_school,
            ad_coaches(&foley, &foley_id, "175", &foley_url, OBSERVED_ON),
        ))
        .expect("json");
        for office in [
            "cogross@apps.isd51.org",
            "Corri Gross",
            "Joel Foss",
            "Daniel Posthumus",
        ] {
            assert!(
                !foley_emitted.contains(office),
                "{office} must not be emitted"
            );
        }

        for label in [
            "AD Administrative Assistant",
            "Activities Director Secretary",
            "Principal",
            "Superintendent",
            "Athletic Trainer/Medical",
            "Band Director",
            "Choir Director",
            "Orchestra Director",
            "Yearbook Advisor",
            "Newspaper Advisor",
            "Title IX Officer",
            "Boys Sports Representative",
            "Girls Sports Representative",
            "Business Manager",
        ] {
            assert_eq!(ad_role(label), None, "{label} is not an AD role");
        }
        assert_eq!(
            ad_role("Activities Director:"),
            Some(CoachRole::AthleticDirector)
        );
        assert_eq!(
            ad_role("Assistant Activities Director"),
            Some(CoachRole::AthleticDirector)
        );
    }

    #[test]
    fn published_phones_and_consumer_mailboxes_never_reach_the_store() {
        let detail = parse_school_detail(WAYZATA);
        let domains = school_domains(&detail);
        assert_eq!(
            domains,
            vec![
                "wayzataschools.org".to_string(),
                "wayzatatrojans.org".to_string()
            ]
        );
        let nodes = select_team_nodes(&parse_team_nodes(WAYZATA_TEAMS));
        let team = nodes
            .iter()
            .find(|node| node.alias.contains("track-and-field-boys"))
            .expect("track and field node");
        let html_url = school_page_url("wayzata-high-school");
        let rows = parse_school_list(LISTING);
        let row = row_for(&rows, "wayzata-high-school", &detail);
        let (school, school_id) =
            school_entities(&row, &detail, &html_url, OBSERVED_ON).expect("school");
        let teams = vec![TeamCoaches {
            node: team.clone(),
            api_url: format!("{COACH_API_PREFIX}{}", team.nid),
            records: parse_coach_records(WAYZATA_TF_COACHES),
        }];
        let coaches = coach_entities(&teams, &school_id, &domains, OBSERVED_ON);
        assert_eq!(
            coaches.len(),
            10,
            "ten real records, all of them MSHSL levels"
        );
        let serialized = serde_json::to_string(&(school, coaches.clone())).expect("json");
        for leak in [
            "763-745-6995",
            "763-745-6889",
            "612-387-2904",
            "6124239766",
            "giesen21@hotmail.com",
            "mike95asmith@gmail.com",
            "tel:",
            "field_work_phone",
        ] {
            assert!(!serialized.contains(leak), "{leak} must not be emitted");
        }
        assert!(coaches.iter().all(|coach| coach.phone.is_none()));
        let head = coaches
            .iter()
            .find(|coach| coach.role == CoachRole::HeadCoach)
            .expect("head coach");
        assert_eq!(head.name, "Aaron Berndt");
        assert_eq!(head.sport, Some(Sport::OutdoorTrack));
        assert_eq!(head.gender, Gender::Boys);
        assert_eq!(
            head.professional_email.as_deref(),
            Some("aaron.berndt@wayzataschools.org")
        );
        assert_eq!(
            head.evidence.len(),
            2,
            "the coach payload and the team page"
        );
    }

    #[test]
    fn team_nodes_filter_to_track_and_cross_country() {
        let nodes = parse_team_nodes(WAYZATA_TEAMS);
        assert_eq!(nodes.len(), 39, "Wayzata publishes 39 team nodes");
        let selected = select_team_nodes(&nodes);
        let picks: Vec<(&str, &str)> = selected
            .iter()
            .map(|node| (node.nid.as_str(), node.alias.as_str()))
            .collect();
        assert_eq!(
            picks,
            vec![
                (
                    "589008",
                    "/schools/wayzata-high-school/cross-country-running-boys/2026"
                ),
                (
                    "589009",
                    "/schools/wayzata-high-school/cross-country-running-girls/2026"
                ),
                (
                    "589034",
                    "/schools/wayzata-high-school/track-and-field-boys/2027"
                ),
                (
                    "589035",
                    "/schools/wayzata-high-school/track-and-field-girls/2027"
                ),
            ]
        );
        assert_eq!(
            team_sport("/schools/x/cross-country-running-girls/2026"),
            Some((Sport::CrossCountry, Gender::Girls))
        );
        assert_eq!(
            team_sport("/schools/x/track-and-field-boys/2027"),
            Some((Sport::OutdoorTrack, Gender::Boys))
        );
        assert_eq!(team_sport("/schools/x/football/2026"), None);
        assert_eq!(team_sport(""), None);
        assert_eq!(
            selected.first().and_then(TeamNode::page_url).as_deref(),
            Some(
                "https://www.mshsl.org/schools/wayzata-high-school/cross-country-running-boys/2026"
            )
        );
    }

    #[test]
    fn coach_levels_map_to_roles_and_only_school_domains_survive() {
        assert!(is_published_level("Head Coach"));
        assert!(!is_published_level("Non-MSHSL Coach"));
        assert!(!is_published_level("mshsl sub-coach"));
        assert_eq!(coach_role("Head Coach"), Some(CoachRole::HeadCoach));
        assert_eq!(
            coach_role("Assistant Coach"),
            Some(CoachRole::AssistantCoach)
        );
        assert_eq!(coach_role("Volunteer Coach"), None);
        assert_eq!(coach_role("Non-MSHSL Coach"), None);
        assert_eq!(coach_role("MSHSL Sub-Coach"), None);

        let domains = vec!["wayzataschools.org".to_string()];
        assert_eq!(
            accept_coach_email("mark.popp@wayzataschools.org", &domains).as_deref(),
            Some("mark.popp@wayzataschools.org")
        );
        assert_eq!(accept_coach_email("giesen21@hotmail.com", &domains), None);
        assert_eq!(
            accept_coach_email("coach@mail.wayzataschools.org", &domains).as_deref(),
            Some("coach@mail.wayzataschools.org")
        );
        assert_eq!(
            accept_coach_email("coach@other-district.org", &domains),
            None
        );
        assert_eq!(accept_coach_email("not-an-address", &domains), None);
        assert_eq!(
            accept_coach_email("aaron.berndt@wayzataschools.org", &[]),
            None,
            "no reference domain, no email"
        );

        // The unexercised branch: a record whose level is not published and a non-MSHSL level record.
        let node = TeamNode {
            nid: "1".to_string(),
            title: "Wayzata High School Track and Field, Boys".to_string(),
            alias: "/schools/wayzata-high-school/track-and-field-boys/2027".to_string(),
        };
        let teams = vec![TeamCoaches {
            node,
            api_url: format!("{COACH_API_PREFIX}1"),
            records: vec![
                CoachRecord {
                    name: "Unpublished Coach".to_string(),
                    level: "Non-MSHSL Coach".to_string(),
                    email: Some("p@example.com".to_string()),
                },
                CoachRecord {
                    name: "Sub Coach".to_string(),
                    level: "MSHSL Sub-Coach".to_string(),
                    email: Some("s@wayzataschools.org".to_string()),
                },
                CoachRecord {
                    name: "Volunteer Coach".to_string(),
                    level: "Volunteer Coach".to_string(),
                    email: None,
                },
            ],
        }];
        let school_id = CanonicalSchool::mint(
            "MN",
            "Wayzata High School",
            &normalize_name("Wayzata High School"),
        );
        assert!(coach_entities(&teams, &school_id, &domains, OBSERVED_ON).is_empty());
    }

    #[test]
    fn ad_email_fill_rate_on_the_fixtures() {
        // Measured over the four school captures: AD rows and how many carry a decoded address.
        let mut rows_total = 0usize;
        let mut with_email = 0usize;
        let school_rows = parse_school_list(LISTING);
        for (html, slug) in [
            (AITKIN, "aitkin-high-school"),
            (FOLEY, "foley-high-school"),
            (WAYZATA, "wayzata-high-school"),
            (ACADEMIC_ARTS, "academic-arts-high-school"),
        ] {
            let detail = parse_school_detail(html);
            let row = row_for(&school_rows, slug, &detail);
            let (_, school_id) =
                school_entities(&row, &detail, &school_page_url(slug), OBSERVED_ON)
                    .expect("school");
            let coaches = ad_coaches(
                &detail,
                &school_id,
                "x",
                &school_page_url(slug),
                OBSERVED_ON,
            );
            rows_total += coaches.len();
            with_email += coaches
                .iter()
                .filter(|coach| coach.professional_email.is_some())
                .count();
        }
        assert_eq!(rows_total, 7, "seven AD rows across the four captures");
        assert_eq!(with_email, 6, "six of them publish an address (85.7%)");
    }

    #[test]
    fn malformed_payloads_yield_zero_rows_instead_of_panicking() {
        assert!(parse_school_list("<html><body>no rows</body></html>").is_empty());
        assert!(parse_school_list("").is_empty());
        assert!(parse_next_listing_page("", 0).is_none());
        assert!(parse_next_listing_page("<a href=\"?page=abc\">", 0).is_none());
        assert!(parse_next_listing_page("<a href=\"/schools/foo\">next</a>", 0).is_none());
        assert!(parse_team_nodes("not json").is_empty());
        assert!(
            parse_team_nodes("{\"data\":[{\"attributes\":{\"title\":\"no nid\"}}]}").is_empty()
        );
        assert!(parse_coach_records("{\"error\":\"not an array\"}").is_empty());
        assert!(parse_coach_records("").is_empty());
        assert!(parse_admin_entries("<html></html>").is_empty());
        assert!(decode_cfemail_fragment("").is_empty());
        let empty = parse_school_detail("");
        assert_eq!(empty, SchoolDetail::default());
        assert!(ad_coaches(
            &empty,
            &CanonicalSchool::mint("MN", "Empty", "empty"),
            "x",
            "u",
            OBSERVED_ON
        )
        .is_empty());
        assert!(school_domains(&empty).is_empty());
        assert!(select_team_nodes(&[]).is_empty());
    }

    #[test]
    fn honorifics_are_stripped_before_minting() {
        assert_eq!(strip_honorific("Mr. Barry Mink"), "Barry Mink");
        assert_eq!(strip_honorific("Coach Jane Doe"), "Jane Doe");
        assert_eq!(strip_honorific("Dr. A. Smith"), "A. Smith");
        assert_eq!(strip_honorific("Matthew Bueckers"), "Matthew Bueckers");
        assert_eq!(strip_honorific("   "), "");
    }

    #[tokio::test]
    async fn collect_fetches_parses_appends_journals_and_reports_from_a_warm_cache() {
        let dir = tempfile::tempdir().expect("temp dir");
        let cache = dir.path().join("http");
        let teams_url = format!(
            "{TEAMS_VIEW_URL}?views-argument%5B%5D=7&fields%5Bnode--participant%5D=title,path,drupal_internal__nid"
        );
        seed_cache(&cache, &listing_page_url(0), LISTING_FIRST_PAGE);
        seed_cache(&cache, &school_page_url("aitkin-high-school"), AITKIN);
        seed_cache(&cache, &teams_url, AITKIN_TEAMS);
        seed_cache(&cache, &format!("{COACH_API_PREFIX}593477"), AITKIN_TF_BOYS);
        seed_cache(
            &cache,
            &format!("{COACH_API_PREFIX}593478"),
            AITKIN_TF_GIRLS,
        );

        let store = crate::store::Store::open(dir.path().join("store")).expect("store");
        let fetcher = crate::net::Fetcher::new(
            &cache,
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
            limit: Some(1),
            refresh: false,
            observed_on: OBSERVED_ON.to_string(),
            states: Vec::new(),
            school_names: vec!["Aitkin High School".to_string()],
        };
        let report = collect(&ctx, &options)
            .await
            .expect("collect returns a report");

        assert_eq!(
            report.rows, 1,
            "one school processed, the other seven rows filtered out"
        );
        assert_eq!(report.errors, 0);
        assert_eq!(
            report.requests, 0,
            "every response came from the seeded cache"
        );
        assert_eq!(
            report.from_cache, 5,
            "listing page, school page, team list and the two track coach payloads"
        );
        assert_eq!(
            report.with_email, 4,
            "two ADs and the two head coaches carry an address"
        );
        assert!(
            report
                .notes
                .iter()
                .any(|note| note.contains("office roles") && note.contains("not emitted")),
            "the report states that office roles were parsed and rejected: {:?}",
            report.notes
        );

        let schools = store
            .scan::<CanonicalSchool>(Table::Schools)
            .expect("schools log");
        assert_eq!(schools.len(), 1);
        assert_eq!(schools[0].name, "Aitkin High School");
        assert_eq!(schools[0].state.as_deref(), Some("MN"));
        assert_eq!(schools[0].city.as_deref(), Some("Aitkin"));
        assert_eq!(schools[0].association.as_deref(), Some("mshsl"));
        assert_eq!(schools[0].enrollment, Some(291));

        let coaches = store
            .scan::<CanonicalCoach>(Table::Coaches)
            .expect("coaches log");
        assert_eq!(
            coaches.len(),
            4,
            "two AD rows plus one head coach per track team"
        );
        let emails: Vec<&str> = coaches
            .iter()
            .filter_map(|coach| coach.professional_email.as_deref())
            .collect();
        assert!(emails.contains(&"jhenrickson@isd1.org"));
        assert!(emails.contains(&"ahills@isd1.org"));
        assert!(emails.contains(&"acarlson@isd1.org"));
        assert!(emails.contains(&"avacarlson@isd1.org"));
        assert!(
            !emails
                .iter()
                .any(|address| address.contains("jforbord") || address.contains("jlong")),
            "the Non-MSHSL Coach records on the same payloads are dropped: {emails:?}"
        );
        let ads: Vec<&CanonicalCoach> = coaches
            .iter()
            .filter(|coach| coach.role == CoachRole::AthleticDirector)
            .collect();
        assert_eq!(ads.len(), 2);
        assert!(ads
            .iter()
            .all(|coach| coach.sport.is_none() && coach.gender == Gender::Mixed));
        let heads: Vec<&CanonicalCoach> = coaches
            .iter()
            .filter(|coach| coach.role == CoachRole::HeadCoach)
            .collect();
        assert_eq!(heads.len(), 2);
        assert!(heads
            .iter()
            .all(|coach| coach.sport == Some(Sport::OutdoorTrack)));
        assert!(heads
            .iter()
            .any(|coach| coach.name == "Adam Carlson" && coach.gender == Gender::Boys));
        assert!(heads
            .iter()
            .any(|coach| coach.name == "Ava Carlson" && coach.gender == Gender::Girls));
        let stored = serde_json::to_string(&coaches).expect("json");
        // The same Administration block publishes the principal, superintendent, advisors and trainer:
        // none of them may be stored as a coach or an athletic director.
        for office in [
            "Lisa DeMars",
            "Dan Stifter",
            "Taylor Meeks",
            "Jennifer Johnson",
            "Briana Tetrick",
            "Jason Henke",
            "Marc Carley",
        ] {
            assert!(
                !stored.contains(office),
                "{office} is an office role and must not be stored"
            );
        }
        assert!(
            coaches.iter().all(|coach| coach.phone.is_none()),
            "no phone column is parsed"
        );

        assert_eq!(
            store.journal_keys("mshsl_schools").expect("journal"),
            HashSet::from([String::from("MN:aitkin-high-school")])
        );
        assert_eq!(
            store.journal_keys("mshsl_coaches").expect("journal"),
            HashSet::from([String::from("MN:aitkin-high-school")])
        );

        // A re-run resumes: the journalled school is skipped and nothing is appended twice.
        let second = collect(&ctx, &options).await.expect("second collect");
        assert_eq!(second.rows, 0, "the school was already journalled");
        assert_eq!(second.requests, 0);
        assert_eq!(
            store
                .scan::<CanonicalSchool>(Table::Schools)
                .expect("schools log")
                .len(),
            1
        );
        assert_eq!(
            store
                .scan::<CanonicalCoach>(Table::Coaches)
                .expect("coaches log")
                .len(),
            4
        );
    }
}
