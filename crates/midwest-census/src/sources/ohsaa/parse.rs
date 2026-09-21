//! Pure parsing: text primitives, the school search page, and coach-name cleaning.
//!
//! Everything here takes captured text and returns parsed rows, so the module is
//! fixture-testable and reusable by the durable services.

use super::map::SearchResult;
use crate::model::{normalize_name, Sport};
use std::collections::{BTreeMap, HashSet};

// ── String primitives ──────────────────────────────────────────────────────

pub(super) fn collapse_whitespace(value: &str) -> String {
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
pub(super) fn strip_tags(fragment: &str) -> String {
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
pub(super) fn decode_entities(value: &str) -> String {
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

pub(super) fn nonempty(value: &str) -> Option<String> {
    let v = collapse_whitespace(value);
    if v.is_empty() {
        None
    } else {
        Some(v)
    }
}

// ── Email validation ───────────────────────────────────────────────────────

pub(super) fn valid_email(value: &str) -> Option<String> {
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
pub(super) fn sport_key(sport: &Sport) -> &str {
    match sport {
        Sport::CrossCountry => "xc",
        Sport::OutdoorTrack => "tf",
        Sport::IndoorTrack => "itf",
    }
}
