use census_domain::model::Gender;
use census_domain::model::Sport;

#[derive(Debug, Clone)]
pub struct SchoolLink {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct CoachRow {
    pub sport_label: String,
    pub name: String,
    pub email: String,
}

#[derive(Debug, Clone, Default)]
pub struct SchoolProfile {
    pub name: String,
    pub address: String,
    pub district: String,
    pub classification: String,
    pub region: String,
    pub phone: String,
    pub coaches: Vec<CoachRow>,
}

fn strip_tags(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut in_tag = false;
    for ch in s.chars() {
        if ch == '<' {
            in_tag = true;
        } else if ch == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(ch);
        }
    }
    result
}

fn trim_ws(s: &str) -> String {
    s.trim().to_string()
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let double = format!("{name}=\"");
    if let Some((_, rest)) = tag.split_once(double.as_str()) {
        return rest.split_once('"').map(|(value, _)| value);
    }
    let single = format!("{name}='");
    if let Some((_, rest)) = tag.split_once(single.as_str()) {
        return rest.split_once('\'').map(|(value, _)| value);
    }
    None
}

fn anchor_body(chunk: &str) -> Option<&str> {
    let (_, rest) = chunk.split_once('>')?;
    Some(rest.split_once("</a>").map_or(rest, |(body, _)| body))
}

fn cell_body(chunk: &str) -> Option<&str> {
    let (_, rest) = chunk.split_once('>')?;
    Some(rest.split_once("</td>").map_or(rest, |(body, _)| body))
}

fn row_cells(row: &str) -> Vec<&str> {
    let mut cells = Vec::new();
    for chunk in row.split("<td").skip(1) {
        let Some(cell) = cell_body(chunk) else {
            continue;
        };
        cells.push(cell);
        if cells.len() == 3 {
            break;
        }
    }
    cells
}

fn mailto_email(cell: &str) -> Option<String> {
    let (_, rest) = cell.split_once("mailto:")?;
    let end = match rest.find(['"', '\'']) {
        Some(index) => index,
        None => rest.len(),
    };
    Some(rest.get(..end)?.trim().to_string())
}

fn url_of(raw_url: &str) -> Option<String> {
    let url = raw_url.trim();
    if url.starts_with("../") || url.starts_with("http") {
        Some(url.to_string())
    } else {
        None
    }
}

pub fn parse_directory_links(html: &str) -> Vec<SchoolLink> {
    let mut links: Vec<SchoolLink> = Vec::new();

    for chunk in html.split("<a ").skip(1) {
        let Some((tag, _)) = chunk.split_once('>') else {
            continue;
        };
        let Some(raw_url) = attribute(tag, "href") else {
            continue;
        };
        if !raw_url.contains("school-directory/?id=") {
            continue;
        }
        let Some(url) = url_of(raw_url) else {
            continue;
        };
        let Some(name) = attribute(tag, "title")
            .or_else(|| attribute(tag, "data-school"))
            .map(trim_ws)
        else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        if links.iter().any(|link| link.url == url) {
            continue;
        }
        links.push(SchoolLink { name, url });
    }

    links
}

pub fn parse_coaches_from_profile(html: &str) -> Vec<CoachRow> {
    let mut rows = Vec::new();

    for chunk in html.split("<tr").skip(1) {
        let Some((_, body)) = chunk.split_once('>') else {
            continue;
        };
        let row = body.split_once("</tr>").map_or(body, |(row, _)| row);
        let cells = row_cells(row);
        let Some(sport_cell) = cells.first() else {
            continue;
        };
        let sport_label = trim_ws(&strip_tags(sport_cell));
        if sport_label.is_empty() {
            continue;
        }
        let Some(coach_cell) = cells.iter().find(|cell| cell.contains("mailto:")) else {
            continue;
        };
        let Some(email) = mailto_email(coach_cell) else {
            continue;
        };
        let name = coach_name(coach_cell);
        if name.is_empty() || is_placeholder_name(&name) {
            continue;
        }
        rows.push(CoachRow {
            sport_label,
            name,
            email,
        });
    }

    rows
}

fn coach_name(cell: &str) -> String {
    let Some((_, rest)) = cell.split_once("<a ") else {
        return String::new();
    };
    let Some(body) = anchor_body(rest) else {
        return String::new();
    };
    trim_ws(&strip_tags(body))
}

pub fn parse_school_profile(html: &str) -> SchoolProfile {
    let mut profile = SchoolProfile::default();

    if let Some(name) = school_name(html) {
        profile.name = name;
    }

    if let Some(ul) = details_list(html) {
        for item in ul.split("</li>") {
            let text = trim_ws(&strip_tags(item));
            if let Some(rest) = text.strip_prefix("District: ") {
                profile.district = rest.to_string();
            } else if let Some(rest) = text.strip_prefix("Classification: ") {
                profile.classification = rest.to_string();
            } else if let Some(rest) = text.strip_prefix("Region: ") {
                profile.region = rest.to_string();
            } else if let Some(rest) = text.strip_prefix("Phone: ") {
                profile.phone = rest.to_string();
            } else if profile.address.is_empty() && text.contains(',') && !text.contains(':') {
                profile.address = text;
            }
        }
    }

    profile.coaches = parse_coaches_from_profile(html);
    profile
}

fn school_name(html: &str) -> Option<String> {
    let (_, rest) = html.split_once("class=\"school-name\"")?;
    let (_, body) = rest.split_once('>')?;
    let (name, _) = body.split_once("</h1>")?;
    let name = trim_ws(&strip_tags(name));
    (!name.is_empty()).then_some(name)
}

fn details_list(html: &str) -> Option<&str> {
    let (_, rest) = html.split_once("class=\"school-details\"")?;
    let (_, body) = rest.split_once('>')?;
    body.split_once("</ul>").map(|(ul, _)| ul)
}

pub fn is_placeholder_name(name: &str) -> bool {
    let name = name.trim().to_lowercase();
    matches!(
        name.as_str(),
        "unknown" | "tba" | "vacant" | "no team" | "no team." | "tbd" | "tbd."
    )
}

pub fn parse_sport_label(label: &str) -> Option<Sport> {
    let l = label.trim().to_lowercase();
    if l.contains("cross country") {
        Some(Sport::CrossCountry)
    } else if l.contains("indoor track") {
        Some(Sport::IndoorTrack)
    } else if l.contains("outdoor track")
        || l.contains("track & field")
        || l.contains("track and field")
    {
        Some(Sport::OutdoorTrack)
    } else {
        None
    }
}

pub fn parse_gender(label: &str) -> Gender {
    let l = label.trim().to_lowercase();
    if l.starts_with("boys") {
        Gender::Boys
    } else if l.starts_with("girls") {
        Gender::Girls
    } else {
        Gender::Mixed
    }
}
