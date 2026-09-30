use crate::CrawlError;
use crate::CrawlResult;

mod schema;
use schema::parse_school_json;
pub use schema::MemberSchool;

pub struct SchoolCoachRow {
    pub school_name: String,
    pub person: String,
    pub activity_name: String,
    pub title: String,
}

pub fn parse_directory(body: &str) -> CrawlResult<Vec<MemberSchool>> {
    let anchor = "[{\\\"schoolCode\\\"";
    let start = body.find(anchor).ok_or(CrawlError::Invariant {
        detail: "directory array marker not found".to_string(),
    })?;
    let from_anchor = body.get(start..).unwrap_or_default();
    let plain = from_anchor.replace("\\\"", "\"");
    let array = extract_balanced_array(&plain)?;
    let schools = parse_school_json(&array)?;
    let schools: Vec<MemberSchool> = schools
        .into_iter()
        .filter(|s| {
            s.name
                .as_deref()
                .map(|n| !n.trim().is_empty())
                .unwrap_or(false)
        })
        .collect();
    Ok(schools)
}

pub fn parse_school_page(body: &str) -> CrawlResult<Vec<SchoolCoachRow>> {
    let school_name = extract_school_name(body);
    if school_name.is_empty() {
        return Err(CrawlError::Invariant {
            detail: "school page carries no title naming the school".to_string(),
        });
    }
    let unescaped = body.replace("\\\"", "\"").replace("\\\\", "\\");
    let rows = extract_coach_rows(&unescaped, &school_name)?;
    let deduped = deduplicate_rows(rows);
    Ok(deduped)
}

fn extract_school_name(body: &str) -> String {
    let Some(open) = body.find("<title>") else {
        return String::new();
    };
    let Some(rest) = body.get(open.saturating_add("<title>".len())..) else {
        return String::new();
    };
    let Some(close) = rest.find("</title>") else {
        return String::new();
    };
    let Some(title) = rest.get(..close) else {
        return String::new();
    };
    let name = title.split('|').next().unwrap_or(title);
    let name = html_unescape(name);
    name.split_whitespace().collect::<Vec<&str>>().join(" ")
}

fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}

fn deduplicate_rows(rows: Vec<SchoolCoachRow>) -> Vec<SchoolCoachRow> {
    let mut seen = std::collections::HashSet::new();
    let mut result = Vec::new();
    for row in rows {
        let key = (
            row.person.clone(),
            row.activity_name.clone(),
            row.title.clone(),
        );
        if !seen.contains(&key) {
            seen.insert(key);
            result.push(row);
        }
    }
    result
}

fn extract_balanced_array(s: &str) -> CrawlResult<String> {
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return Err(CrawlError::Invariant {
            detail: "empty input to array extractor".to_string(),
        });
    }
    let mut depth: usize = 0;
    let mut in_string = false;
    let mut escaped = false;
    for (i, &ch) in chars.iter().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        if ch == '"' {
            in_string = true;
        } else if ch == '[' || ch == '{' {
            depth = depth.saturating_add(1);
        } else if ch == ']' || ch == '}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                let end = i.saturating_add(1);
                let Some(result) = chars.get(..end) else {
                    return Err(CrawlError::Invariant {
                        detail: "unbalanced directory array".to_string(),
                    });
                };
                return Ok(result.iter().collect());
            }
        }
    }
    Err(CrawlError::Invariant {
        detail: "unbalanced directory array".to_string(),
    })
}

fn extract_coach_rows(text: &str, school_name: &str) -> CrawlResult<Vec<SchoolCoachRow>> {
    let activity_anchor = r#"{"activityName":"#;
    let objects = objects_at_anchor(text, activity_anchor)?;
    let mut result = Vec::new();
    for obj in &objects {
        let activity_name = obj
            .get("activityName")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .unwrap_or_default();
        let members = obj.get("members").and_then(|v| v.as_array());
        if let Some(members) = members {
            for member in members {
                let person = member
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                if person.is_empty() {
                    continue;
                }
                let positions = member.get("positions").and_then(|v| v.as_array());
                if let Some(positions) = positions {
                    for position in positions {
                        let title = position
                            .get("title")
                            .and_then(|v| v.as_str())
                            .map(|s| s.trim().to_string())
                            .unwrap_or_default();
                        result.push(SchoolCoachRow {
                            school_name: school_name.to_string(),
                            person: person.clone(),
                            activity_name: activity_name.clone(),
                            title,
                        });
                    }
                }
            }
        }
    }
    Ok(result)
}

fn objects_at_anchor(text: &str, anchor: &str) -> CrawlResult<Vec<serde_json::Value>> {
    let mut results = Vec::new();
    let mut cursor = 0usize;
    while let Some(rest) = text.get(cursor..) {
        let Some(offset) = rest.find(anchor) else {
            break;
        };
        let start = cursor.saturating_add(offset);
        let Some(object) = text.get(start..) else {
            break;
        };
        let mut stream =
            serde_json::Deserializer::from_str(object).into_iter::<serde_json::Value>();
        let consumed = match stream.next() {
            Some(Ok(value)) => {
                results.push(value);
                stream.byte_offset()
            }
            Some(Err(source)) => {
                return Err(CrawlError::Decode {
                    url: "chsaanow.com/schools/<slug>/".to_string(),
                    source,
                })
            }
            None => {
                return Err(CrawlError::Invariant {
                    detail: "activity anchor carries no JSON value".to_string(),
                })
            }
        };
        cursor = start.saturating_add(consumed);
    }
    Ok(results)
}
