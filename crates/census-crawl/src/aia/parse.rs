use census_domain::model::Gender;
use thiserror::Error;

const NAME_MARK: &str =
    "text-4xl font-extrabold tracking-tight leading-none md:text-5xl xl:text-6xl\">";
const CARD: &str = "<div class=\"md:flex px-4 py-2";
const COACH_SIDE: &str = "<div class=\"md:w-2/3\">";
const TEXT_MARK: &str = "leading-none text-lg font-semibold mb-1\">";
const ROLE_MARK: &str = "text-slate-500 dark:text-slate-400 mb-2\">";
const ADDRESS: &str = "<address class=\"mt-6 mb-6\">";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchoolInfo {
    pub name: String,
    pub full_name: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoachRow {
    pub sport_label: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchoolProfile {
    pub info: SchoolInfo,
    pub coaches: Vec<CoachRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRow {
    pub school_id: String,
    pub name: String,
    pub full_name: Option<String>,
    pub city: Option<String>,
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("aia parsing failed: {reason}")]
    Failed { reason: String },
}

pub fn parse_search_json(document: &str) -> Result<Vec<ParsedRow>, ParseError> {
    let value: serde_json::Value =
        serde_json::from_str(document).map_err(|e| ParseError::Failed {
            reason: e.to_string(),
        })?;

    let arr = value.as_array().ok_or(ParseError::Failed {
        reason: "expected JSON array".to_string(),
    })?;

    let mut rows = Vec::new();
    arr.iter().try_for_each(|item| {
        rows.try_reserve(1).map_err(|_| ParseError::Failed {
            reason: "search row allocation failed".into(),
        })?;
        rows.push(parse_search_item(item)?);
        Ok::<_, ParseError>(())
    })?;
    Ok(rows)
}

pub(super) fn parse_search_item(item: &serde_json::Value) -> Result<ParsedRow, ParseError> {
    let obj = item.as_object().ok_or_else(|| ParseError::Failed {
        reason: "expected JSON object".into(),
    })?;
    let school_id = obj
        .get("id")
        .and_then(serde_json::Value::as_i64)
        .map(|id| id.to_string())
        .ok_or_else(|| ParseError::Failed {
            reason: "missing school id".into(),
        })?;
    let name = obj
        .get("name")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| ParseError::Failed {
            reason: "missing school name".into(),
        })?;
    let full_name = obj
        .get("full_name")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    let city = obj
        .get("address")
        .and_then(|address| address.get("city"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned);
    Ok(ParsedRow {
        school_id,
        name,
        full_name,
        city,
    })
}

pub fn parse_school_profile(document: &str) -> Result<SchoolProfile, ParseError> {
    let info = parse_school_info(document)?;
    let coaches = parse_coach_rows(document);
    Ok(SchoolProfile { info, coaches })
}

pub(super) fn parse_school_info(document: &str) -> Result<SchoolInfo, ParseError> {
    let name = text_between(document, NAME_MARK, "</h2>")
        .filter(|name| !name.is_empty())
        .ok_or(ParseError::Failed {
            reason: "school name heading not found".to_string(),
        })?;
    Ok(SchoolInfo {
        name,
        full_name: None,
        address: find_address(document),
    })
}

pub(super) fn parse_coach_rows(document: &str) -> Vec<CoachRow> {
    let mut rows = Vec::new();
    for card in document.split(CARD).skip(1) {
        let Some(side_at) = card.find(COACH_SIDE) else {
            continue;
        };
        let (left, side) = card.split_at(side_at);
        let Some(sport_label) = text_between(left, TEXT_MARK, "</div>") else {
            continue;
        };
        if sport_label.is_empty() {
            continue;
        }
        if let Some(name) = head_coach(side) {
            rows.push(CoachRow { sport_label, name });
        }
    }
    rows
}

fn head_coach(side: &str) -> Option<String> {
    let mut rest = side;
    while let Some(start) = rest.find(TEXT_MARK) {
        let after = rest.get(start.saturating_add(TEXT_MARK.len())..)?;
        let stop = after.find("</div>")?;
        let name = decode(after.get(..stop)?.trim());
        let tail = after.get(stop..)?;
        if let Some(role_at) = tail.find(ROLE_MARK) {
            if let Some(role_rest) = tail.get(role_at.saturating_add(ROLE_MARK.len())..) {
                if let Some(role_end) = role_rest.find("</div>") {
                    if role_rest.get(..role_end)?.contains("Head Coach")
                        && !name.is_empty()
                        && !is_placeholder_name(&name)
                    {
                        return Some(name);
                    }
                }
            }
        }
        rest = tail;
    }
    None
}

fn text_between(document: &str, start: &str, end: &str) -> Option<String> {
    let at = document.find(start)?;
    let rest = document.get(at.saturating_add(start.len())..)?;
    let stop = rest.find(end)?;
    let slice = rest.get(..stop)?;
    Some(decode(slice.trim()))
}

fn find_address(document: &str) -> Option<String> {
    let at = document.find(ADDRESS)?;
    let rest = document.get(at.saturating_add(ADDRESS.len())..)?;
    let stop = rest.find("</address>")?;
    let block = rest.get(..stop)?;
    let lines = match block.find("</div>") {
        Some(cut) => match block.get(cut.saturating_add("</div>".len())..) {
            Some(lines) => lines,
            None => block,
        },
        None => block,
    };
    let text = lines
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("<br>", "\n");
    let text = decode(&text);
    let text = text.trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

fn decode(text: &str) -> String {
    text.replace("&#039;", "'")
        .replace("&#39;", "'")
        .replace("&quot;", "\"")
        .replace("&ndash;", "-")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

fn is_placeholder_name(name: &str) -> bool {
    let name = name.trim().to_lowercase();
    matches!(
        name.as_str(),
        "" | "unknown" | "tba" | "tbd" | "vacant" | "no team" | "no team." | "no.team"
    )
}

pub fn parse_sport_label(label: &str) -> Option<census_domain::model::Sport> {
    let l = label.trim().to_ascii_lowercase();
    match l.as_str() {
        "cross country - boy's"
        | "cross country - girl's"
        | "boys cross country"
        | "girls cross country" => Some(census_domain::model::Sport::CrossCountry),
        "track - boy's"
        | "track - girl's"
        | "track & field - boy's"
        | "track & field - girl's"
        | "boys outdoor track"
        | "girls outdoor track"
        | "boys track"
        | "girls track" => Some(census_domain::model::Sport::OutdoorTrack),
        "boys indoor track" | "girls indoor track" => {
            Some(census_domain::model::Sport::IndoorTrack)
        }
        _ => None,
    }
}

pub fn parse_gender(label: &str) -> Gender {
    let l = label.trim().to_ascii_lowercase();
    if l.contains("boy") {
        Gender::Boys
    } else if l.contains("girl") {
        Gender::Girls
    } else {
        Gender::Mixed
    }
}
