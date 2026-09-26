//! Pure parsing: the school directory and the per-school staff table.
//!
//! Captured text in, parsed rows out; no I/O, no store access.

/// The class on the div that wraps one school's directory entry, and the marker the walk steps by.
const SCHOOL_LIST_WRAPPER: &str = "SchoolListWrapper";
/// The query key whose value is the FusionPoint school id, on the anchor that owns a wrapper.
const SCHOOL_ID: &str = "SchoolID=";
/// The school page's staff table, spelled with its whole opening tag. The class name alone also
/// appears in the page's `style` block, where `.DirectoryStaffTable { … }` names a rule and not a
/// table, so a search for the bare class finds the stylesheet first.
const SCHOOL_STAFF_TABLE: &str = "<table class='DirectoryStaffTable'>";

/// One school entry from the directory page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchoolEntry {
    /// Display name as published (e.g. "Bangor High School").
    pub name: String,
    /// FusionPoint school ID used for the staff page URL.
    pub school_id: String,
}

/// One coach row extracted from a staff table row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoachRow {
    /// Sport label as published (e.g. "Boys Cross Country").
    pub sport: String,
    /// Head-coach name.
    pub coach: String,
}

/// Parse the directory page into school entries.
pub fn parse_directory(html: &str) -> Vec<SchoolEntry> {
    let mut entries = Vec::new();
    let mut cursor = 0usize;

    while let Some(rest) = html.get(cursor..) {
        let Some(found) = rest.find(SCHOOL_LIST_WRAPPER) else {
            break;
        };
        let Some(wrapper_start) = cursor.checked_add(found) else {
            break;
        };
        let Some(wrapper) = html.get(wrapper_start..) else {
            break;
        };
        let span = wrapper
            .get(SCHOOL_LIST_WRAPPER.len()..)
            .and_then(|after| after.find(SCHOOL_LIST_WRAPPER))
            .and_then(|next| SCHOOL_LIST_WRAPPER.len().checked_add(next))
            .unwrap_or(wrapper.len());
        let wrapper = wrapper.get(..span).unwrap_or_default();

        let school_id = html
            .get(..wrapper_start)
            .and_then(|before| before.rfind(SCHOOL_ID))
            .and_then(|at| at.checked_add(SCHOOL_ID.len()))
            .and_then(|start| school_id_at(html, start));
        let name = wrapper_name(wrapper);

        if let (Some(school_id), Some(name)) = (school_id, name) {
            entries.push(SchoolEntry { name, school_id });
        }

        match wrapper_start.checked_add(SCHOOL_LIST_WRAPPER.len()) {
            Some(next) if next > cursor => cursor = next,
            _ => break,
        }
    }

    entries
}

/// The digits that follow a `SchoolID=` occurrence, or `None` when none do.
fn school_id_at(html: &str, start: usize) -> Option<String> {
    let digits: String = html
        .get(start..)?
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect();
    if digits.is_empty() {
        None
    } else {
        Some(digits)
    }
}

/// The `<p>` text inside one wrapper's `SchoolListName` div.
fn wrapper_name(wrapper: &str) -> Option<String> {
    let at = wrapper.find("<p>")?;
    let text = wrapper.get(at..)?.strip_prefix("<p>")?;
    let end = text.find("</p>")?;
    let name = decode_html(text.get(..end)?.trim());
    if name.is_empty() {
        None
    } else {
        Some(name)
    }
}

/// Parse the staff table from an individual school page.
///
/// Returns coach rows for every Head Coach entry in the table.
pub fn parse_staff_table(html: &str) -> Vec<CoachRow> {
    let Some(table_start) = html.find(SCHOOL_STAFF_TABLE) else {
        return Vec::new();
    };
    let mut rest = html.get(table_start..).unwrap_or_default();

    let mut rows = Vec::new();
    while let Some(tr_start) = rest.find("<tr>") {
        let Some(from_row) = rest.get(tr_start..) else {
            break;
        };
        let Some(tr_end) = from_row.find("</tr>") else {
            break;
        };
        let Some(row) = from_row.get(..tr_end) else {
            break;
        };
        let Some(after_row) = from_row
            .get(tr_end..)
            .and_then(|tail| tail.strip_prefix("</tr>"))
        else {
            break;
        };
        rest = after_row;

        let cells = extract_cells(row);
        let [sport_cell, role_cell, coach_cell, ..] = cells.as_slice() else {
            continue;
        };
        let sport = decode_html(sport_cell.trim());
        let role = decode_html(role_cell.trim());
        let coach = decode_html(coach_cell.trim());

        if role == "Head Coach" && !sport.is_empty() && !coach.is_empty() {
            rows.push(CoachRow { sport, coach });
        }
    }

    rows
}

/// Extract all <td> cell texts from a row.
fn extract_cells(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut rest = row;

    while let Some(td_start) = rest.find("<td") {
        let Some(from_cell) = rest.get(td_start..) else {
            break;
        };
        let Some(td_end) = from_cell.find("</td>") else {
            break;
        };
        let Some(cell_html) = from_cell.get(..td_end) else {
            break;
        };
        cells.push(strip_html(cell_html));
        let Some(after_cell) = from_cell
            .get(td_end..)
            .and_then(|tail| tail.strip_prefix("</td>"))
        else {
            break;
        };
        rest = after_cell;
    }

    cells
}

/// Remove HTML tags, keeping the text between them.
fn strip_html(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

/// Decode common HTML entities.
fn decode_html(html: &str) -> String {
    html.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}
