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
use crate::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use crate::sources::{AdapterContext, AdapterReport};
use crate::store::Table;
use anyhow::Result;
use std::collections::{BTreeMap, HashSet};

/// Host serving the OHSAA officials portal.
pub const HOST: &str = "https://officials.myohsaa.org";
/// Evidence/adapter slug used in [`SourceRef`]s.
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

// ── Parsed shapes ──────────────────────────────────────────────────────────

/// One row of the search result table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchResult {
    /// ALL-CAPS school name as published (e.g. "DUBLIN COFFMAN").
    pub name: String,
    /// City from the second column (e.g. "Dublin").
    pub city: String,
    /// OHSAA numeric school id (e.g. "474").
    pub ohsaa_id: String,
}

impl SearchResult {
    pub fn page_url(&self) -> String {
        format!("{HOST}{SCHOOL_INFO_PATH}?ohsaaId={}", self.ohsaa_id)
    }
    pub fn sports_url(&self) -> String {
        format!("{HOST}{SPORTS_PATH}?ohsaaId={}", self.ohsaa_id)
    }
    pub fn ad_url(&self) -> String {
        format!("{HOST}{AD_PATH}?ohsaaId={}", self.ohsaa_id)
    }
}

/// One coach extracted from a sports-information cell.
#[derive(Debug, Clone)]
pub struct CoachEntry {
    pub name: String,
    pub email: Option<String>,
}

/// Parsed content from an athletic department page.
#[derive(Debug, Clone, Default)]
pub struct AdPage {
    pub director: Option<(String, Option<String>)>,
    pub office_roles: Vec<(String, String)>,
}

/// Canonical entities for one school.
#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub school_id: SchoolId,
    pub coaches: Vec<CanonicalCoach>,
}

// ── String primitives ──────────────────────────────────────────────────────

fn collapse_whitespace(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut prev_space = false;
    for ch in value.chars() {
        if ch.is_whitespace() {
            if !prev_space {
                result.push(' ');
                prev_space = true;
            }
        } else {
            prev_space = false;
            result.push(ch);
        }
    }
    result.trim().to_string()
}

/// Remove HTML tags, decode entities, and collapse whitespace.
fn strip_tags(fragment: &str) -> String {
    let mut result = String::with_capacity(fragment.len());
    let mut in_tag = false;
    let mut bytes = fragment.as_bytes().iter().copied().peekable();
    while let Some(byte) = bytes.next() {
        match byte {
            b'<' => in_tag = true,
            b'>' => {
                in_tag = false;
                if bytes.peek().is_some_and(|next| *next != b' ') {
                    result.push(' ');
                }
            }
            other if !in_tag => result.push(char::from(other)),
            _ => {}
        }
    }
    collapse_whitespace(&result)
}

/// Decode `&amp;`, `&lt;`, `&gt;`, `&quot;`, `&#39;`, and `&#NNN;`.
fn decode_entities(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '&' {
            // Collect entity name up to semicolon (explicit loop avoids
            // take_while's off-by-one with borrowed iterators)
            let mut rest = String::new();
            let mut found_semicolon = false;
            loop {
                match chars.next() {
                    Some(';') => {
                        found_semicolon = true;
                        break;
                    }
                    Some(c) => rest.push(c),
                    None => {
                        break;
                    }
                }
            }
            match rest.as_str() {
                "amp" => result.push('&'),
                "lt" => result.push('<'),
                "gt" => result.push('>'),
                "quot" => result.push('"'),
                "apos" => result.push('\''),
                _ => {
                    if found_semicolon {
                        if let Ok(code) = rest.parse::<u32>() {
                            if let Some(c) = char::from_u32(code) {
                                result.push(c);
                            } else {
                                result.push('&');
                                result.push_str(&rest);
                                result.push(';');
                            }
                        } else {
                            result.push('&');
                            result.push_str(&rest);
                            result.push(';');
                        }
                    } else {
                        // Incomplete entity (no semicolon found): preserve as-is
                        result.push('&');
                        result.push_str(&rest);
                    }
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}

fn nonempty(value: &str) -> Option<String> {
    let v = collapse_whitespace(value);
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

// ── Email validation ───────────────────────────────────────────────────────

fn valid_email(value: &str) -> Option<String> {
    let v = value.trim();
    if v.is_empty() {
        return None;
    }
    let (local, domain) = v.split_once('@')?;
    if !local.is_empty() && !domain.is_empty() && domain.contains('.') {
        Some(v.to_string())
    } else {
        None
    }
}

// ── Search parsing ─────────────────────────────────────────────────────────

/// Parse the school search result table.
///
/// Returns unique rows deduplicated by `ohsaaId`. The OHSAA search page emits
/// multiple identical rows for every autocomplete suggestion that matches.
pub fn parse_search(html: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let mut seen_ids: HashSet<String> = HashSet::new();

    // Rows are the `</tr>`-terminated segments in document order; a `<tr>` with no closing tag
    // ends the scan, which is what the previous cursor walk did.
    for chunk in html.split_inclusive("</tr>") {
        let Some(body) = chunk.strip_suffix("</tr>") else {
            break;
        };
        let Some(open) = body.find("<tr>") else {
            continue;
        };
        let Some(row) = body.get(open..) else {
            continue;
        };

        let Some(id) = extract_ohsaa_id(row) else {
            continue;
        };
        if seen_ids.insert(id.clone()) {
            results.push(SearchResult {
                name: extract_cell_text(row, 0),
                city: extract_cell_text(row, 1),
                ohsaa_id: id,
            });
        }
    }

    results
}

fn extract_ohsaa_id(row: &str) -> Option<String> {
    let (_, after) = row.split_once("ohsaaId=")?;
    let end = after
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after.len());
    let id = after.get(..end)?;
    if !id.is_empty() {
        return Some(id.to_string());
    }
    None
}

fn extract_cell_text(row: &str, index: usize) -> String {
    let Some((start, _)) = row.match_indices("<td>").nth(index) else {
        return String::new();
    };
    let Some(after_td) = row.get(start..).and_then(|tail| tail.strip_prefix("<td>")) else {
        return String::new();
    };
    match after_td.split_once("</td>") {
        Some((cell, _)) => decode_entities(&strip_tags(cell)),
        None => String::new(),
    }
}

/// Resolve a school name to a unique OHSAA id using the search results.
///
/// Returns the first unique `SearchResult` matching the normalised name,
/// or `None` if no match. Notes ambiguity in the provided vector.
pub fn resolve_school_name(
    search_html: &str,
    query: &str,
    notes: &mut Vec<String>,
) -> Option<SearchResult> {
    let results = parse_search(search_html);
    if results.is_empty() {
        return None;
    }

    let normalized = normalize_name(query);
    let mut by_id: BTreeMap<String, &SearchResult> = BTreeMap::new();
    for r in &results {
        by_id.entry(r.ohsaa_id.clone()).or_insert(r);
    }

    let mut exact_matches: Vec<&SearchResult> = Vec::new();
    for r in by_id.values() {
        if normalize_name(&r.name) == normalized {
            exact_matches.push(r);
        }
    }

    if exact_matches.len() > 1 {
        notes.push(format!(
            "ambiguous name \"{}\": {} distinct schools share the normalised name, using first match",
            query, exact_matches.len()
        ));
    }
    if let Some(first) = exact_matches.first() {
        return Some((**first).clone());
    }

    if results.len() == 1 {
        return results.into_iter().next();
    }

    let unique_ids: HashSet<&str> = by_id.keys().map(|s| s.as_str()).collect();
    notes.push(format!(
        "query \"{}\": {} result rows but {} unique schools; using first match",
        query,
        results.len(),
        unique_ids.len()
    ));
    results.into_iter().next()
}

// ── Sports parsing ─────────────────────────────────────────────────────────

/// Map a sport label to a Sport variant.
pub fn parse_sport_label(label: &str) -> Option<Sport> {
    let cleaned = collapse_whitespace(&decode_entities(label));
    match cleaned.as_str() {
        "Cross Country" => Some(Sport::CrossCountry),
        "Track & Field" | "Track &amp; Field" => Some(Sport::OutdoorTrack),
        _ => None,
    }
}

/// Parse a coach cell (e.g. `<a href="mailto:..." class="fieldValue">Joe DePalma (Div-I)</a>`).
///
/// Returns `None` for "N/A", "TBA", or empty cells.
pub fn parse_coach_cell(cell_html: &str) -> Option<CoachEntry> {
    let cleaned = collapse_whitespace(&decode_entities(&strip_tags(cell_html)));
    if cleaned == "N/A" || cleaned.starts_with("TBA") || cleaned.is_empty() {
        return None;
    }

    let name = if let Some((before_div, _)) = cleaned.split_once(" (Div-") {
        strip_honorific(before_div.trim().trim_end_matches(','))
    } else {
        strip_honorific(&cleaned)
    };
    if name.is_empty() {
        return None;
    }

    let email = if let Some((_, after)) = cell_html.split_once("href=\"mailto:") {
        let end = after.find('"').unwrap_or(after.len());
        after.get(..end).and_then(valid_email)
    } else if let Some((_, after)) = cell_html.split_once("href='mailto:") {
        let end = after.find('\'').unwrap_or(after.len());
        after.get(..end).and_then(valid_email)
    } else {
        None
    };

    Some(CoachEntry { name, email })
}

/// Parse the sports-information table for TF/XC sections.
///
/// Returns tuples of (sport_label, boys_coach, girls_coach).
pub fn parse_sports_table(html: &str) -> Vec<(String, Option<CoachEntry>, Option<CoachEntry>)> {
    let Some(table_start) = html.find("informationSportHeaderRow") else {
        return Vec::new();
    };
    let Some(table) = html.get(table_start..) else {
        return Vec::new();
    };

    let mut sections = Vec::new();
    for chunk in table.split_inclusive("</tr>") {
        // A `<tr` that never closes cannot yield a row; the previous cursor walk stopped there.
        if chunk.strip_suffix("</tr>").is_none() {
            break;
        }
        let Some(open) = chunk.find("<tr") else {
            continue;
        };
        let Some(row) = chunk.get(open..) else {
            continue;
        };

        if row.contains("informationSportHeaderRow") {
            continue;
        }

        let sport_raw = extract_td_text(row, 0);
        let boys_raw = extract_td_text(row, 1);
        let girls_raw = extract_td_text(row, 2);

        let sport_label = strip_tags(&decode_entities(&sport_raw)).trim().to_string();
        if parse_sport_label(&sport_label).is_some() {
            let boys = parse_coach_cell(&boys_raw);
            let girls = parse_coach_cell(&girls_raw);
            if boys.is_some() || girls.is_some() {
                sections.push((sport_label, boys, girls));
            }
        }
    }

    sections
}

/// Extract text from the Nth <td> in a row (no tag stripping yet).
fn extract_td_text(row: &str, index: usize) -> String {
    let mut count = 0;
    let mut rest = row;
    while let Some(open) = rest.find("<td") {
        let Some(from_td) = rest.get(open..) else {
            break;
        };
        let Some((text, after)) = from_td.split_once("</td>") else {
            break;
        };
        if count == index {
            return text.to_string();
        }
        // The previous cursor moved past `</td>`, so a `<td` nested inside a cell is not a cell.
        rest = after;
        count = count.saturating_add(1);
    }
    String::new()
}

/// Parse the athletic department table.
///
/// Returns the Athletic Director (first "Athletic Director:" row) and skips
pub fn parse_ad_page(html: &str) -> AdPage {
    let mut ad = AdPage::default();
    // Collect the body rows; separator rows (`<br>`) carry no labels or values.
    let mut rows: Vec<&str> = Vec::new();
    for chunk in html.split_inclusive("</tr>") {
        // A `<tr` that never closes ends the walk, as the previous cursor loop did.
        if chunk.strip_suffix("</tr>").is_none() {
            break;
        }
        let Some(open) = chunk.find("<tr") else {
            continue;
        };
        let Some(row) = chunk.get(open..) else {
            continue;
        };
        if row.contains("<br") {
            continue;
        }
        rows.push(row);
    }
    // The association writes label rows followed by value rows: a row of
    // `athleticDepartmentSubheader` spans names the role (and repeats an `Email:` label), the next
    // row carries the person and their mailto link. Labels therefore stay pending until a row with
    // a name appears.
    let mut pending: Vec<String> = Vec::new();
    for row in rows.iter().copied() {
        let labels: Vec<String> = extract_subheader_labels(row)
            .into_iter()
            .filter(|l| !is_value_label(l))
            .collect();
        if !labels.is_empty() {
            pending = labels;
        }

        let name = extract_field_name(row);
        if name.is_empty() {
            continue;
        }
        let email = extract_field_email(row);
        let role = pending.iter().find_map(|label| classify_role(label));
        pending.clear();
        match role {
            Some("athletic director") if ad.director.is_none() => {
                ad.director = Some((name, email));
            }
            Some(label) => ad.office_roles.push((label.to_string(), name)),
            None => {
                // A name under a label we do not track (or under no label at all).
            }
        }
    }

    ad
}

/// `true` when a subheader names a value column rather than a person role.
///
/// Phone and fax labels arrive with the number inside the same span (`Phone: (614) 718-8142`).
fn is_value_label(label: &str) -> bool {
    let lowered = label.trim().to_ascii_lowercase();
    lowered.starts_with("email") || lowered.starts_with("phone") || lowered.starts_with("fax")
}

/// Map a subheader label to a role, or `None` when it names no person role.
///
/// Labels carry trailing colons and the association writes plural forms
/// (`Assistant Athletic Secretaries:`), so the match is prefix-based after stripping the colon.
fn classify_role(label: &str) -> Option<&'static str> {
    let lowered = label
        .trim()
        .trim_end_matches(':')
        .trim()
        .to_ascii_lowercase();
    if lowered.starts_with("assistant athletic director") {
        Some("assistant athletic director")
    } else if lowered.starts_with("assistant athletic secretar") {
        Some("assistant athletic secretary")
    } else if lowered.starts_with("athletic secretar") {
        Some("athletic secretary")
    } else if lowered.starts_with("athletic trainer") {
        Some("athletic trainer")
    } else if lowered.starts_with("principal") {
        Some("principal")
    } else if lowered.starts_with("superintendent") {
        Some("superintendent")
    } else if lowered.starts_with("business manager") {
        Some("business manager")
    } else if lowered == "athletic director" {
        Some("athletic director")
    } else {
        None
    }
}

/// Extract subheader labels (span with athleticDepartmentSubheader class) from a label row.
fn extract_subheader_labels(row: &str) -> Vec<String> {
    let mut labels = Vec::new();
    let mut iter = 0usize;
    let mut rest = row;
    while let Some(start) = rest.find("<span") {
        iter = iter.saturating_add(1);
        if iter > 1000 {
            break;
        }
        let Some(from_span) = rest.get(start..) else {
            break;
        };
        let Some((span, after)) = from_span.split_once("</span>") else {
            break;
        };
        if span.contains("athleticDepartmentSubheader") {
            let body = span.split_once('>').map_or("", |(_, body)| body);
            labels.push(decode_entities(body.trim()));
        }
        rest = after;
    }
    labels
}

/// Extract the name value (first <span class="fieldValue">) from a data row.
fn extract_field_name(row: &str) -> String {
    let Some((_, after)) = row.split_once("<span class=\"fieldValue\">") else {
        return String::new();
    };
    match after.split_once("</span>") {
        Some((raw, _)) => decode_entities(&strip_tags(raw)),
        None => String::new(),
    }
}

/// Extract the email from a mailto: href in the data row.
fn extract_field_email(row: &str) -> Option<String> {
    if let Some((_, after)) = row.split_once("href=\"mailto:") {
        let end = after.find('"').unwrap_or(after.len());
        after.get(..end).and_then(valid_email)
    } else if let Some((_, after)) = row.split_once("href='mailto:") {
        let end = after.find('\'').unwrap_or(after.len());
        after.get(..end).and_then(valid_email)
    } else {
        None
    }
}

// ── Name cleaning ──────────────────────────────────────────────────────────

/// Strip leading honorifics so "Coach Barry Mink" and "Barry Mink" mint the same coach.
pub fn strip_honorific(value: &str) -> String {
    let trimmed = value.trim();
    let lower = trimmed.to_lowercase();
    for prefix in &["coach ", "mr. ", "mrs. ", "ms. ", "dr. ", "prof. "] {
        if lower.starts_with(*prefix) {
            let rest = trimmed.get(prefix.len()..).unwrap_or(trimmed).trim();
            return if rest.is_empty() {
                trimmed.to_string()
            } else {
                rest.to_string()
            };
        }
    }
    trimmed.to_string()
}

/// Short sport label for identity key.
fn sport_key(sport: &Sport) -> &str {
    match sport {
        Sport::CrossCountry => "xc",
        Sport::OutdoorTrack => "tf",
        Sport::IndoorTrack => "itf",
    }
}

// ── Entity mapping ─────────────────────────────────────────────────────────

/// Build canonical school and coach entities from a school's search result,
/// sports page, and AD page.
pub fn school_entities(
    result: &SearchResult,
    sports_html: &str,
    ad_html: &str,
    observed_on: &str,
) -> SchoolExtract {
    let city = nonempty(&result.city);
    let page_url = result.page_url();
    let sports_url = result.sports_url();
    let ad_url = result.ad_url();

    // Build school
    let (mut school, school_id) =
        CanonicalSchool::new(STATE, &result.name, normalize_name(&result.name));
    school.city = city;
    school.association = Some(ASSOCIATION.to_string());
    school.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: "ohsaa".into(),
            },
            &result.ohsaa_id,
        )
        .with_url(page_url.clone()),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(page_url.clone())),
        observed_on.to_string(),
    ));

    // Collect coaches
    let mut coaches: Vec<CanonicalCoach> = Vec::new();

    // Parse AD
    let ad = parse_ad_page(ad_html);
    if let Some((ad_name, ad_email)) = ad.director {
        let clean_name = strip_honorific(&ad_name);
        let mut coach = CanonicalCoach::new(
            &school_id,
            clean_name,
            None, // AD is school-wide
            Gender::Mixed,
            CoachRole::AthleticDirector,
        );
        coach.professional_email = ad_email;
        coach.source_identities.push(
            SourceIdentity::new(
                SourceNamespace::AssociationSchool {
                    association: "ohsaa".into(),
                },
                format!("ad:{}", result.ohsaa_id),
            )
            .with_url(ad_url.clone()),
        );
        coach.evidence.push(Evidence::parsed(
            SourceRef::new(SOURCE_ID, Some(ad_url)),
            observed_on.to_string(),
        ));
        coaches.push(coach);
    }

    // Parse sports sections
    let sections = parse_sports_table(sports_html);

    for (sport_label, boys, girls) in sections {
        if let Some(sport) = parse_sport_label(&sport_label) {
            // Boys coach
            if let Some(boys_entry) = boys {
                let clean_name = strip_honorific(&boys_entry.name);
                let mut coach = CanonicalCoach::new(
                    &school_id,
                    clean_name,
                    Some(sport),
                    Gender::Boys,
                    CoachRole::HeadCoach,
                );
                coach.professional_email = boys_entry.email;
                coach.source_identities.push(
                    SourceIdentity::new(
                        SourceNamespace::AssociationSchool {
                            association: "ohsaa".into(),
                        },
                        format!("coach:{}:{}:boys", result.ohsaa_id, sport_key(&sport)),
                    )
                    .with_url(sports_url.clone()),
                );
                coach.evidence.push(Evidence::parsed(
                    SourceRef::new(SOURCE_ID, Some(sports_url.clone())),
                    observed_on.to_string(),
                ));
                coaches.push(coach);
            }

            // Girls coach
            if let Some(girls_entry) = girls {
                let clean_name = strip_honorific(&girls_entry.name);
                let mut coach = CanonicalCoach::new(
                    &school_id,
                    clean_name,
                    Some(sport),
                    Gender::Girls,
                    CoachRole::HeadCoach,
                );
                coach.professional_email = girls_entry.email;
                coach.source_identities.push(
                    SourceIdentity::new(
                        SourceNamespace::AssociationSchool {
                            association: "ohsaa".into(),
                        },
                        format!("coach:{}:{}:girls", result.ohsaa_id, sport_key(&sport)),
                    )
                    .with_url(sports_url.clone()),
                );
                coach.evidence.push(Evidence::parsed(
                    SourceRef::new(SOURCE_ID, Some(sports_url.clone())),
                    observed_on.to_string(),
                ));
                coaches.push(coach);
            }
        }
    }

    SchoolExtract {
        school,
        school_id,
        coaches,
    }
}

// ── Collection ─────────────────────────────────────────────────────────────

/// Collect OHSAA schools and coaches.
///
/// Resumable: a school page is fetched only when `OH:<ohsaaId>` is absent from
/// the `ohsaa_schools` journal. Both journals carry the same key.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> Result<AdapterReport> {
    let mut report = AdapterReport::new("ohsaa", "schools");
    let observed_on = if options.observed_on.trim().is_empty() {
        ctx.observed_on.clone()
    } else {
        options.observed_on.clone()
    };
    let before = ctx.fetcher.stats().await;

    // Restrict to OH state
    if !options.states.is_empty() && !options.states.iter().any(|s| s.eq_ignore_ascii_case("OH")) {
        let after = ctx.fetcher.stats().await;
        report.requests = after.requests.saturating_sub(before.requests);
        report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
        report.note(format!(
            "states {:?} do not include OH; this adapter covers Ohio only",
            options.states
        ));
        return Ok(report);
    }

    // Resolve schools
    let mut to_process: Vec<SearchResult> = if !options.school_names.is_empty() {
        let mut results = Vec::new();
        for name in &options.school_names {
            let url = format!("{HOST}{SEARCH_PATH}?Name={}", url_encode(name));
            let mut notes: Vec<String> = Vec::new();
            match ctx.fetcher.get(&url, &ctx.fetch_options()).await {
                Ok(outcome) if outcome.status == 200 => {
                    if let Some(sr) = resolve_school_name(&outcome.text(), name, &mut notes) {
                        results.push(sr);
                    } else {
                        report.note(format!("no school found for \"{}\"", name));
                    }
                    for n in notes {
                        report.note(n);
                    }
                }
                Ok(outcome) => {
                    report.errors = report.errors.saturating_add(1);
                    report.note(format!(
                        "search \"{}\" returned HTTP {}",
                        name, outcome.status
                    ));
                }
                Err(e) => {
                    report.errors = report.errors.saturating_add(1);
                    report.note(format!("search \"{}\": {}", name, e));
                }
            }
        }
        results
    } else {
        // Read existing OH schools from schools.jsonl
        let existing = crate::report::read_rows::<crate::model::CanonicalSchool>(
            &ctx.store.out_dir().join("schools.jsonl"),
        )?;
        existing
            .into_iter()
            .filter(|s| {
                matches!(
                    &s.association,
                    Some(ref a) if a.as_str() == "ohsaa"
                )
            })
            .map(|s| SearchResult {
                name: s.name,
                city: s.city.clone().unwrap_or_default(),
                ohsaa_id: s
                    .source_identities
                    .iter()
                    .find_map(|si| match &si.namespace {
                        SourceNamespace::AssociationSchool { .. } => Some(&si.id),
                        _ => None,
                    })
                    .cloned()
                    .unwrap_or_default(),
            })
            .collect()
    };

    // Deduplicate by ohsaaId
    let mut seen_ids: HashSet<String> = HashSet::new();
    to_process.retain(|sr| seen_ids.insert(sr.ohsaa_id.clone()));

    // Check journal for already-processed schools
    let done = ctx.store.journal_keys("ohsaa_schools")?;
    to_process.retain(|sr| !done.contains(&format!("OH:{}", sr.ohsaa_id)));

    // Apply limit
    if let Some(limit) = options.limit {
        to_process.truncate(limit);
    }

    let total_schools = to_process.len();

    let mut processed = 0usize;
    let skipped_done = 0usize;
    let mut not_found = 0usize;
    let mut fetch_failures = 0usize;
    let mut coach_rows = 0usize;
    let mut with_email = 0u64;
    let mut office_roles_skipped = 0usize;

    for sr in &to_process {
        let sports_url = sr.sports_url();
        let ad_url = sr.ad_url();
        let school_key = format!("OH:{}", sr.ohsaa_id);

        // Fetch sports page
        let sports_result = ctx
            .fetcher
            .get(
                &sports_url,
                &crate::net::FetchOptions {
                    allow_not_found: true,
                    ..ctx.fetch_options()
                },
            )
            .await;

        let sports_html = match sports_result {
            Ok(outcome) if outcome.status == 200 => outcome.text(),
            Ok(outcome) if outcome.status == 404 => {
                not_found = not_found.saturating_add(1);
                report.note(format!(
                    "school {} (ID {}) returned 404",
                    sr.name, sr.ohsaa_id
                ));
                continue;
            }
            Ok(outcome) => {
                fetch_failures = fetch_failures.saturating_add(1);
                report.note(format!(
                    "school {} sports returned HTTP {}",
                    sr.name, outcome.status
                ));
                continue;
            }
            Err(e) => {
                fetch_failures = fetch_failures.saturating_add(1);
                report.note(format!("school {} sports: {}", sr.name, e));
                continue;
            }
        };

        // Fetch AD page
        let ad_result = ctx
            .fetcher
            .get(
                &ad_url,
                &crate::net::FetchOptions {
                    allow_not_found: true,
                    ..ctx.fetch_options()
                },
            )
            .await;

        let ad_html = match ad_result {
            Ok(outcome) if outcome.status == 200 => outcome.text(),
            Ok(outcome) => {
                report.note(format!(
                    "school {} AD page returned HTTP {}",
                    sr.name, outcome.status
                ));
                String::new()
            }
            Err(e) => {
                report.note(format!("school {} AD page: {}", sr.name, e));
                String::new()
            }
        };

        // Parse and emit entities
        let extract = school_entities(sr, &sports_html, &ad_html, &observed_on);

        // Count office roles skipped
        let ad = parse_ad_page(&ad_html);
        office_roles_skipped = office_roles_skipped.saturating_add(ad.office_roles.len());

        // Append school
        ctx.store.append(Table::Schools, &extract.school)?;
        report.rows = report.rows.saturating_add(1);

        // Append coaches
        let mut coach_emails = 0u64;
        for coach in &extract.coaches {
            ctx.store.append(Table::Coaches, coach)?;
            coach_rows = coach_rows.saturating_add(1);
            if coach.professional_email.is_some() {
                coach_emails = coach_emails.saturating_add(1);
            }
        }
        with_email = with_email.saturating_add(coach_emails);

        // Journal both schools and coaches
        ctx.store.journal_done(
            "ohsaa_schools",
            &school_key,
            &serde_json::json!({
                "ohsaa_id": sr.ohsaa_id,
                "city": sr.city,
                "coaches": extract.coaches.len(),
            }),
        )?;

        // Journal each coach under the same key
        for coach in &extract.coaches {
            ctx.store.journal_done(
                "ohsaa_coaches",
                &school_key,
                &serde_json::json!({
                    "coach_name": coach.name,
                    "sport": format!("{:?}", coach.sport),
                    "gender": format!("{:?}", coach.gender),
                    "role": format!("{:?}", coach.role),
                    "email": coach.professional_email.is_some(),
                }),
            )?;
        }

        processed = processed.saturating_add(1);
    }

    let after = ctx.fetcher.stats().await;
    report.requests = after.requests.saturating_sub(before.requests);
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    report.with_email = with_email;

    report.note(format!(
        "processed {} of {} requested schools ({} skipped_done, {} not_found, {} fetch_failures, {} coach_rows, {} office_roles_skipped)",
        processed, total_schools, skipped_done, not_found, fetch_failures, coach_rows, office_roles_skipped
    ));

    Ok(report)
}

/// URL-encode a school name for the search query parameter.
fn url_encode(value: &str) -> String {
    let mut result = String::new();
    for ch in value.chars() {
        match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(ch),
            ' ' => result.push_str("%20"),
            _ => {
                for byte in ch.to_string().bytes() {
                    result.push_str(&format!("%{:02X}", byte));
                }
            }
        }
    }
    result
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Fixture data ─────────────────────────────────────────────────────

    // Trimmed Dublin Coffman search: one unique row
    fn fixture_search_dublin() -> &'static str {
        include_str!("../../tests/fixtures/ohsaa/search_dublin_coffman.html")
    }

    // Dublin Coffman sports page (all sports, XC + T&F present)
    fn fixture_sports_dublin() -> &'static str {
        include_str!("../../tests/fixtures/ohsaa/sports_dublin_coffman.html")
    }

    // Centerville sports (XC + T&F boys, girls TBA)
    fn fixture_sports_centerville() -> &'static str {
        include_str!("../../tests/fixtures/ohsaa/sports_centerville.html")
    }

    // Dublin Coffman AD page (AD + assistant AD + secretaries)
    fn fixture_ad_dublin() -> &'static str {
        include_str!("../../tests/fixtures/ohsaa/ad_dublin_coffman.html")
    }

    // Centerville AD page (AD + assistant AD)
    fn fixture_ad_centerville() -> &'static str {
        include_str!("../../tests/fixtures/ohsaa/ad_centerville.html")
    }

    // Search with duplicate rows (same ohsaaId repeated)
    fn fixture_search_duplicates() -> &'static str {
        include_str!("../../tests/fixtures/ohsaa/search_duplicate_rows.html")
    }

    // Search with no results table
    fn fixture_search_no_results() -> &'static str {
        include_str!("../../tests/fixtures/ohsaa/search_no_results.html")
    }

    // Malformed sports HTML
    fn fixture_sports_malformed() -> &'static str {
        include_str!("../../tests/fixtures/ohsaa/sports_malformed.html")
    }

    // Malformed AD HTML
    fn fixture_ad_malformed() -> &'static str {
        include_str!("../../tests/fixtures/ohsaa/ad_malformed.html")
    }

    // ── Search parsing tests ─────────────────────────────────────────────

    #[test]
    fn parse_search_returns_unique_schools() {
        let results = parse_search(fixture_search_dublin());
        assert_eq!(results.len(), 1, "should have exactly 1 unique school");
        let r = &results[0];
        assert_eq!(r.name, "DUBLIN COFFMAN", "school name should be ALL-CAPS");
        assert_eq!(r.city, "Dublin", "city should be title-case");
        assert_eq!(r.ohsaa_id, "474", "should match the ohsaaId");
    }

    #[test]
    fn parse_search_deduplicates_rows() {
        let results = parse_search(fixture_search_duplicates());
        assert_eq!(
            results.len(),
            1,
            "all duplicate rows should be deduplicated to 1"
        );
        let r = &results[0];
        assert_eq!(r.name, "MENTOR");
        assert_eq!(r.ohsaa_id, "1016");
    }

    #[test]
    fn parse_search_empty_yields_empty_vec() {
        let results = parse_search(fixture_search_no_results());
        assert!(
            results.is_empty(),
            "search with no results table should return empty vec, not error"
        );
    }

    // ── Sports parsing tests ─────────────────────────────────────────────

    #[test]
    fn parse_sports_table_extracts_xc_coaches() {
        let sections = parse_sports_table(fixture_sports_dublin());
        let xc = sections
            .iter()
            .find(|(label, _, _)| label == "Cross Country");
        assert!(xc.is_some(), "should find Cross Country row");
        let (_, boys, girls) = xc.unwrap();
        let boys = boys.as_ref().unwrap();
        assert_eq!(boys.name, "Joe DePalma");
        assert_eq!(
            boys.email,
            Some("depalma_joseph@dublinschools.net".to_string())
        );
        let girls = girls.as_ref().unwrap();
        assert_eq!(girls.name, "Greg King");
        assert_eq!(girls.email, Some("king_greg@dublinschools.net".to_string()));
    }

    #[test]
    fn parse_sports_table_extracts_track_field() {
        let sections = parse_sports_table(fixture_sports_dublin());
        let tf = sections
            .iter()
            .find(|(label, _, _)| label == "Track & Field");
        assert!(tf.is_some(), "should find Track & Field row");
        let (_, boys, girls) = tf.unwrap();
        let boys = boys.as_ref().unwrap();
        assert_eq!(boys.name, "James Legins");
        assert_eq!(boys.email, Some("j.legins106@gmail.com".to_string()));
        let girls = girls.as_ref().unwrap();
        assert_eq!(girls.name, "Greg King");
        assert_eq!(girls.email, Some("king_greg@dublinschools.net".to_string()));
    }

    #[test]
    fn parse_sports_table_handles_tba() {
        let sections = parse_sports_table(fixture_sports_centerville());
        let tf = sections
            .iter()
            .find(|(label, _, _)| label == "Track & Field");
        assert!(tf.is_some(), "should find Track & Field row");
        let (_, boys, girls) = tf.unwrap();
        assert!(boys.is_some(), "boys coach should be present");
        let boys = boys.as_ref().unwrap();
        assert_eq!(boys.name, "Matt Somerlot");
        assert_eq!(
            boys.email,
            Some("matt.somerlot@centerville.k12.oh.us".to_string())
        );
        assert!(girls.is_none(), "girls coach TBA should parse as None");
    }

    #[test]
    fn parse_sports_table_malformed_yields_empty() {
        let sections = parse_sports_table(fixture_sports_malformed());
        assert!(
            sections.is_empty(),
            "malformed HTML should yield 0 sections, not panic"
        );
    }

    #[test]
    fn parse_coach_cell_skips_na() {
        assert!(parse_coach_cell("N/A").is_none());
        assert!(parse_coach_cell("TBA (Div-I)").is_none());
        assert!(parse_coach_cell("").is_none());
    }

    #[test]
    fn parse_coach_cell_parses_mailto() {
        let cell = r#"<a href="mailto:depalma_joseph@dublinschools.net" class="fieldValue">Joe DePalma (Div-I)</a>"#;
        let coach = parse_coach_cell(cell).expect("should parse");
        assert_eq!(coach.name, "Joe DePalma");
        assert_eq!(
            coach.email,
            Some("depalma_joseph@dublinschools.net".to_string())
        );
    }

    // ── AD parsing tests ─────────────────────────────────────────────────

    #[test]
    fn parse_ad_page_extracts_director() {
        let ad = parse_ad_page(fixture_ad_dublin());
        assert!(ad.director.is_some(), "should find athletic director");
        let (name, email) = ad.director.as_ref().unwrap();
        assert_eq!(name, "Duane Sheldon");
        assert_eq!(email, &Some("sheldon_duane@dublinschools.net".to_string()));
    }

    #[test]
    fn parse_ad_page_excludes_office_roles_from_director() {
        let ad = parse_ad_page(fixture_ad_dublin());
        // Office roles should be in the office_roles list, not as director
        assert_eq!(ad.office_roles.len(), 2, "should have 2 office roles");
        let role_names: Vec<&str> = ad.office_roles.iter().map(|(l, _)| l.as_str()).collect();
        assert!(
            role_names.contains(&"assistant athletic director"),
            "assistant AD should be in office roles"
        );
        assert!(
            role_names.contains(&"assistant athletic secretary"),
            "assistant secretary should be in office roles"
        );
        // Director should NOT be in office roles
        assert!(
            !role_names.contains(&"athletic director"),
            "AD must not appear in office roles list"
        );
    }

    #[test]
    fn parse_ad_page_malformed_yields_empty() {
        let ad = parse_ad_page(fixture_ad_malformed());
        assert!(
            ad.director.is_none(),
            "malformed AD page should not produce a director"
        );
    }

    // ── Entity building tests ────────────────────────────────────────────

    #[test]
    fn school_entities_includes_ad_and_xc_coaches() {
        let sr = SearchResult {
            name: "DUBLIN COFFMAN".to_string(),
            city: "Dublin".to_string(),
            ohsaa_id: "474".to_string(),
        };
        let extract = school_entities(
            &sr,
            fixture_sports_dublin(),
            fixture_ad_dublin(),
            "2026-09-19",
        );

        // School
        assert_eq!(extract.school.name, "DUBLIN COFFMAN");
        assert_eq!(extract.school.city, Some("Dublin".to_string()));
        assert_eq!(extract.school.association, Some("ohsaa".to_string()));

        // Coaches: AD + XC boys + XC girls + TF boys + TF girls = 5
        assert_eq!(extract.coaches.len(), 5, "should have AD + 4 coach rows");

        // Verify coach types
        let roles: Vec<String> = extract
            .coaches
            .iter()
            .map(|c| format!("{:?}", c.role))
            .collect();
        assert!(roles.contains(&"AthleticDirector".to_string()));

        // Verify all coaches have email
        let all_have_email = extract
            .coaches
            .iter()
            .all(|c| c.professional_email.is_some());
        assert!(
            all_have_email,
            "all 5 coaches should have email (100% fill rate)"
        );
    }

    #[test]
    fn school_entities_handles_tba_girls_coach() {
        let sr = SearchResult {
            name: "CENTERVILLE".to_string(),
            city: "Centerville".to_string(),
            ohsaa_id: "336".to_string(),
        };
        let extract = school_entities(
            &sr,
            fixture_sports_centerville(),
            fixture_ad_centerville(),
            "2026-09-19",
        );

        // AD + XC boys + XC girls + TF boys = 4 (girls T&F is TBA)
        assert_eq!(extract.coaches.len(), 4);

        // Verify T&F girls is missing
        let tf_girls = extract
            .coaches
            .iter()
            .find(|c| matches!(c.sport, Some(Sport::OutdoorTrack)) && c.gender == Gender::Girls);
        assert!(tf_girls.is_none(), "T&F girls should not be present (TBA)");
    }

    // ── Honorific stripping tests ────────────────────────────────────────

    #[test]
    fn strip_honorific_removes_prefixes() {
        assert_eq!(strip_honorific("Coach Joe DePalma"), "Joe DePalma");
        assert_eq!(strip_honorific("Mr. Barry Mink"), "Barry Mink");
        assert_eq!(strip_honorific("Mrs. Jane Smith"), "Jane Smith");
        assert_eq!(strip_honorific("Dr. Robert Wolf"), "Robert Wolf");
        assert_eq!(
            strip_honorific("Joe DePalma"),
            "Joe DePalma",
            "no-op when no honorific"
        );
    }

    // ── Office role exclusion tests ──────────────────────────────────────

    #[test]
    fn office_roles_never_imported_as_coaches() {
        // The AD page fixture has Assistant AD (Scott Caster) and
        // Assistant Athletic Secretary (Andrea Guilliams) — these must
        // NOT appear as coaches or athletic directors in the output.
        let sr = SearchResult {
            name: "DUBLIN COFFMAN".to_string(),
            city: "Dublin".to_string(),
            ohsaa_id: "474".to_string(),
        };
        let extract = school_entities(
            &sr,
            fixture_sports_dublin(),
            fixture_ad_dublin(),
            "2026-09-19",
        );

        let ad_names: Vec<&str> = extract
            .coaches
            .iter()
            .filter(|c| c.role == CoachRole::AthleticDirector)
            .map(|c| c.name.as_str())
            .collect();

        assert_eq!(ad_names, vec!["Duane Sheldon"]);
        assert!(
            !ad_names.contains(&"Scott Caster"),
            "Assistant AD must not be imported as AD"
        );
        assert!(
            !ad_names.contains(&"Andrea Guilliams"),
            "Assistant Secretary must not be imported as AD"
        );
    }

    // ── Malformed / empty payload tests ──────────────────────────────────

    #[test]
    fn malformed_search_does_not_panic() {
        let results = parse_search("<html><body><p>No table here</p></body></html>");
        assert!(results.is_empty());
    }

    #[test]
    fn malformed_sports_does_not_panic() {
        let sections = parse_sports_table("<html><body><p>No table</p></body></html>");
        assert!(sections.is_empty());
    }

    #[test]
    fn malformed_ad_does_not_panic() {
        let ad = parse_ad_page("<html><body><p>No table</p></body></html>");
        assert!(ad.director.is_none());
    }

    // ── Sport label mapping tests ────────────────────────────────────────

    #[test]
    fn parse_sport_label_maps_correctly() {
        assert_eq!(
            parse_sport_label("Cross Country"),
            Some(Sport::CrossCountry)
        );
        assert_eq!(
            parse_sport_label("Track & Field"),
            Some(Sport::OutdoorTrack)
        );
        assert_eq!(
            parse_sport_label("Track &amp; Field"),
            Some(Sport::OutdoorTrack)
        );
        assert_eq!(parse_sport_label("Basketball"), None);
        assert_eq!(parse_sport_label(""), None);
    }
}
