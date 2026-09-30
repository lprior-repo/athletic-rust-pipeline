use super::types::SchoolCoachRow;
use crate::CrawlError;
use crate::CrawlResult;

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
    let title_start = match body.find("<title>").and_then(|pos| pos.checked_add(7)) {
        Some(pos) => pos,
        None => return String::new(),
    };
    let title_end = match body
        .get(title_start..)
        .and_then(|rest| rest.find("</title>"))
        .and_then(|pos| title_start.checked_add(pos))
    {
        Some(pos) => pos,
        None => return String::new(),
    };
    let title = match body.get(title_start..title_end) {
        Some(title) => title,
        None => return String::new(),
    };
    let name = title.split('|').next().unwrap_or(title);
    let name = html_unescape(name);
    let name = name.split_whitespace().collect::<Vec<&str>>().join(" ");
    name.trim().to_string()
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

fn extract_coach_rows(text: &str, school_name: &str) -> CrawlResult<Vec<SchoolCoachRow>> {
    let activity_anchor = r#"{"activityName":"#;
    let objects = objects_at_anchor(text, activity_anchor)?;
    let mut result = Vec::new();
    for obj_json in &objects {
        let obj: serde_json::Value =
            serde_json::from_str(obj_json).map_err(|source| CrawlError::Decode {
                url: "chsaanow.com/schools/<slug>/".to_string(),
                source,
            })?;
        let activity_name = obj
            .get("activityName")
            .and_then(|value| value.as_str())
            .map(|text| text.to_string())
            .unwrap_or_default();
        let members = obj.get("members").and_then(|value| value.as_array());
        if let Some(members) = members {
            for member in members {
                let person = member
                    .get("name")
                    .and_then(|value| value.as_str())
                    .map(|text| text.trim().to_string())
                    .unwrap_or_default();
                if person.is_empty() {
                    continue;
                }
                let positions = member.get("positions").and_then(|value| value.as_array());
                if let Some(positions) = positions {
                    for position in positions {
                        let title = position
                            .get("title")
                            .and_then(|value| value.as_str())
                            .map(|text| text.trim().to_string())
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

fn objects_at_anchor(text: &str, anchor: &str) -> CrawlResult<Vec<String>> {
    let mut results = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let anchor_chars: Vec<char> = anchor.chars().collect();
    let anchor_len = anchor_chars.len();
    let mut i: usize = 0;
    while let Some(end) = i.checked_add(anchor_len) {
        if end > n {
            break;
        }
        let mut match_anchor = true;
        for (offset, expected) in anchor_chars.iter().enumerate() {
            let actual = i.checked_add(offset).and_then(|at| chars.get(at));
            if actual != Some(expected) {
                match_anchor = false;
                break;
            }
        }
        if match_anchor {
            let obj = extract_one_object(&chars, i)?;
            results.push(obj);
            i = i.saturating_add(anchor_len).saturating_add(100);
        } else {
            i = i.saturating_add(1);
        }
    }
    Ok(results)
}

fn extract_one_object(chars: &[char], start: usize) -> CrawlResult<String> {
    let mut depth: usize = 0;
    let mut in_string = false;
    let mut escaped = false;
    let tail = chars.get(start..).ok_or(CrawlError::Invariant {
        detail: "coach object start out of range".to_string(),
    })?;
    for (i, &ch) in tail.iter().enumerate() {
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
        } else if ch == '{' {
            depth = depth.saturating_add(1);
        } else if ch == '}' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                let slice = tail.get(..=i).ok_or(CrawlError::Invariant {
                    detail: "coach object boundary out of range".to_string(),
                })?;
                return Ok(slice.iter().collect());
            }
        }
    }
    Err(CrawlError::Invariant {
        detail: "unbalanced coach object".to_string(),
    })
}
