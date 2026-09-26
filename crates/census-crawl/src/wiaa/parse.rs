//! Pure parsing for the WIAA directory: one-letter index rows and one school page.
//!
//! No I/O and no store access: every function takes captured HTML and returns a parsed row
//! shape. Canonical entities are minted in [`super::map`].

use super::primitives::{
    attribute_value, cell_email, clean, element_bodies, element_text, find_from, labelled_texts,
    meaningful, nonempty, nth, nth_owned, strip_tags, table_slice,
};
use super::{HOST, SCHOOL_PATH};

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
