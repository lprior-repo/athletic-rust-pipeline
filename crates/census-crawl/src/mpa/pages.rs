const SCHOOL_LIST_WRAPPER: &str = "SchoolListWrapper";
const SCHOOL_ID: &str = "SchoolID=";
const SCHOOL_STAFF_TABLE: &str = "<table class='DirectoryStaffTable'>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchoolEntry {
    pub name: String,
    pub school_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoachRow {
    pub sport: String,
    pub coach: String,
}

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
            .map_or(wrapper.len(), |value| value);
        let wrapper = wrapper
            .get(..span)
            .map_or(Default::default(), core::convert::identity);

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

pub fn parse_staff_table(html: &str) -> Vec<CoachRow> {
    let Some(table_start) = html.find(SCHOOL_STAFF_TABLE) else {
        return Vec::new();
    };
    let mut rest = html
        .get(table_start..)
        .map_or(Default::default(), core::convert::identity);

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

fn decode_html(html: &str) -> String {
    html.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}
