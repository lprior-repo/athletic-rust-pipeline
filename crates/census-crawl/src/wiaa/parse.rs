
use super::primitives::{
    attribute_value, cell_email, clean, element_bodies, element_text, find_from, labelled_texts,
    meaningful, nonempty, nth, nth_owned, strip_tags, table_slice,
};
use super::{HOST, SCHOOL_PATH};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct IndexEntry {
    pub org_id: String,
    pub name: String,
    pub level: String,
    pub city: String,
}

impl IndexEntry {
    pub fn page_url(&self) -> String {
        format!("{HOST}{SCHOOL_PATH}?orgID={}", self.org_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StaffRow {
    pub role: String,
    pub name: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoachRow {
    pub sport: String,
    pub name: String,
    pub role: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SchoolPage {
    pub name: String,
    pub level: Option<String>,
    pub city: Option<String>,
    pub conference: Option<String>,
    pub enrollment: Option<u32>,
    pub website: Option<String>,
    pub admins: Vec<StaffRow>,
    pub coaches: Vec<CoachRow>,
}

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

fn labeled_value(html: &str, label: &str) -> Option<String> {
    let marker = format!("<span>{label}</span>");
    let start = find_from(html, &marker, 0)?.checked_add(marker.len())?;
    let (value, _) = element_text(html, "h5", start)?;
    meaningful(&value)
}

fn label_text(html: &str, class: &str) -> Option<String> {
    labelled_texts(html, class)
        .into_iter()
        .find_map(|value| meaningful(&value))
}

pub fn parse_enrollment(html: &str) -> Option<u32> {
    let label = "<span>School:</span>";
    let at = find_from(html, "<span>Enrollment (", 0)?;
    let after = find_from(html, label, at)?.checked_add(label.len())?;
    let (value, _) = element_text(html, "b", after)?;
    value.trim().parse::<u32>().ok()
}

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

fn cell_person(cell: &str) -> String {
    element_text(cell, "b", 0)
        .map(|(text, _)| text)
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| strip_tags(cell))
}

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
