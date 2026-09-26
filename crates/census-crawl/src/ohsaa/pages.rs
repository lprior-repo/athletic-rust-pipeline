//! Pure parsing: the per-school sports-information table and athletic-department page.
//!
//! Captured text in, parsed rows out; no I/O, no store access.

use super::map::{AdPage, CoachEntry};
use super::parse::{
    collapse_whitespace, decode_entities, strip_honorific, strip_tags, valid_email,
};
use census_domain::model::Sport;

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

/// Extract text from the Nth `<td>` in a row (no tag stripping yet).
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
    let mut rows: Vec<&str> = Vec::new();
    for chunk in html.split_inclusive("</tr>") {
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
            None => {}
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

/// Extract the name value (first `<span class="fieldValue">`) from a data row.
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
