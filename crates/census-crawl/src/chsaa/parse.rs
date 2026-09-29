use crate::CrawlError;
use crate::CrawlResult;

pub struct MemberSchool {
    pub school_code: Option<u64>,
    pub slug: Option<String>,
    pub name: Option<String>,
    pub official_name: Option<String>,
    pub city: Option<String>,
    pub street_address: Option<String>,
    pub zip_code: Option<String>,
    pub phone: Option<String>,
    pub district_name: Option<String>,
    pub member_type: Option<String>,
    pub school_type: Option<String>,
    pub setting: Option<String>,
}

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
    let from_anchor = &body[start..];
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
    let title_start = match body.find("<title>") {
        Some(pos) => pos + 7,
        None => return String::new(),
    };
    let title_end = match body[title_start..].find("</title>") {
        Some(pos) => title_start + pos,
        None => return String::new(),
    };
    let title = &body[title_start..title_end];
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
                let end = i + 1;
                let result: String = chars[..end].iter().collect();
                return Ok(result);
            }
        }
    }
    Err(CrawlError::Invariant {
        detail: "unbalanced directory array".to_string(),
    })
}

fn parse_school_json(json_str: &str) -> CrawlResult<Vec<MemberSchool>> {
    let raw: serde_json::Value =
        serde_json::from_str(json_str).map_err(|source| CrawlError::Decode {
            url: "chsaanow.com/schools/".to_string(),
            source,
        })?;
    let arr = raw.as_array().ok_or(CrawlError::Schema {
        url: "chsaanow.com/schools/".to_string(),
        detail: "expected array of school records".to_string(),
    })?;
    let mut schools = Vec::with_capacity(arr.len());
    for item in arr {
        let obj = item.as_object().ok_or(CrawlError::Schema {
            url: "chsaanow.com/schools/".to_string(),
            detail: "expected object per school record".to_string(),
        })?;
        let school = decode_school_record(obj)?;
        schools.push(school);
    }
    Ok(schools)
}

fn decode_school_record(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> CrawlResult<MemberSchool> {
    let school_code = decode_u64(obj, "schoolCode");
    let slug = decode_string(obj, "slug");
    let name = decode_string(obj, "name");
    let official_name = decode_string(obj, "officialName");
    let city = decode_string(obj, "city");
    let street_address = decode_string(obj, "streetAddress");
    let zip_code = decode_string(obj, "zipCode");
    let phone = decode_string(obj, "phone");
    let district_name = decode_string(obj, "districtName");
    let member_type = decode_string(obj, "memberType");
    let school_type = decode_string(obj, "schoolType");
    let setting = decode_string(obj, "setting");
    Ok(MemberSchool {
        school_code,
        slug,
        name,
        official_name,
        city,
        street_address,
        zip_code,
        phone,
        district_name,
        member_type,
        school_type,
        setting,
    })
}

fn decode_string(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    obj.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn decode_u64(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<u64> {
    obj.get(key).and_then(|v| v.as_u64())
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

fn objects_at_anchor(text: &str, anchor: &str) -> CrawlResult<Vec<String>> {
    let mut results = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let n = chars.len();
    let anchor_chars: Vec<char> = anchor.chars().collect();
    let anchor_len = anchor_chars.len();
    let mut i = 0;
    while i + anchor_len <= n {
        let mut match_anchor = true;
        for j in 0..anchor_len {
            if chars[i + j] != anchor_chars[j] {
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
    for (i, &ch) in chars[start..].iter().enumerate() {
        let pos = start + i;
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
                let result: String = chars[start..=pos].iter().collect();
                return Ok(result);
            }
        }
    }
    Err(CrawlError::Invariant {
        detail: "unbalanced coach object".to_string(),
    })
}
