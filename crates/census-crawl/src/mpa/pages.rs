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

    while let Some(found) = html[cursor..].find(SCHOOL_LIST_WRAPPER) {
        let wrapper_start = cursor + found;
        let wrapper = &html[wrapper_start..];
        // A wrapper never nests, so its span ends at the next one. Bounding the span here is what
        // stops a wrapper with no `<p>` of its own from borrowing the next school's name.
        let span = wrapper[SCHOOL_LIST_WRAPPER.len()..]
            .find(SCHOOL_LIST_WRAPPER)
            .map_or(wrapper.len(), |next| SCHOOL_LIST_WRAPPER.len() + next);
        let wrapper = &wrapper[..span];

        // The anchor that owns this wrapper closes before it, and its `href` carries the SchoolID:
        // `<a href='/SchoolPages/School.aspx?SchoolID=25'>`. The nearest one to the left is this
        // wrapper's, because the page emits anchor then wrapper, anchor then wrapper.
        let school_id = html[..wrapper_start]
            .rfind(SCHOOL_ID)
            .and_then(|at| school_id_at(html, at + SCHOOL_ID.len()));
        let name = wrapper_name(wrapper);

        if let (Some(school_id), Some(name)) = (school_id, name) {
            entries.push(SchoolEntry { name, school_id });
        }

        cursor = wrapper_start + SCHOOL_LIST_WRAPPER.len();
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
    let text = &wrapper[at + 3..];
    let end = text.find("</p>")?;
    let name = decode_html(text[..end].trim());
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
    let table = &html[table_start..];

    let mut rows = Vec::new();
    // Walk through all <tr> elements in the table.
    let mut rest = table;
    while let Some(tr_start) = rest.find("<tr>") {
        rest = &rest[tr_start..];
        let Some(tr_end) = rest.find("</tr>") else {
            break;
        };
        let row = &rest[..tr_end];
        rest = &rest[tr_end + 5..];

        // Extract cells from the row.
        let cells = extract_cells(row);
        if cells.len() >= 3 {
            let sport = decode_html(cells[0].trim());
            let role = decode_html(cells[1].trim());
            let coach = decode_html(cells[2].trim());

            // Only keep Head Coach rows (skip Principal, AD, etc.)
            if role == "Head Coach" && !sport.is_empty() && !coach.is_empty() {
                rows.push(CoachRow { sport, coach });
            }
        }
    }

    rows
}

/// Extract all <td> cell texts from a row.
fn extract_cells(row: &str) -> Vec<String> {
    let mut cells = Vec::new();
    let mut rest = row;

    while let Some(td_start) = rest.find("<td") {
        rest = &rest[td_start..];
        if let Some(td_end) = rest.find("</td>") {
            let cell_html = &rest[..td_end];
            // Strip HTML tags from the cell.
            let text = strip_html(cell_html);
            cells.push(text);
            rest = &rest[td_end + 5..];
        } else {
            break;
        }
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
