use super::types::MemberSchool;
use crate::CrawlError;
use crate::CrawlResult;

pub fn parse_directory(body: &str) -> CrawlResult<Vec<MemberSchool>> {
    let anchor = "[{\\\"schoolCode\\\"";
    let start = body.find(anchor).ok_or(CrawlError::Invariant {
        detail: "directory array marker not found".to_string(),
    })?;
    let from_anchor = body.get(start..).ok_or(CrawlError::Invariant {
        detail: "directory array marker is not on a character boundary".to_string(),
    })?;
    let plain = from_anchor.replace("\\\"", "\"");
    let array = extract_balanced_array(&plain)?;
    let schools = parse_school_json(&array)?;
    let schools: Vec<MemberSchool> = schools
        .into_iter()
        .filter(|school| {
            school
                .name
                .as_deref()
                .map(|name| !name.trim().is_empty())
                .unwrap_or(false)
        })
        .collect();
    Ok(schools)
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
                let end = i.checked_add(1).ok_or(CrawlError::Invariant {
                    detail: "directory array boundary overflowed".to_string(),
                })?;
                let slice = chars.get(..end).ok_or(CrawlError::Invariant {
                    detail: "directory array boundary out of range".to_string(),
                })?;
                return Ok(slice.iter().collect());
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
        .and_then(|value| value.as_str())
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
}

fn decode_u64(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<u64> {
    obj.get(key).and_then(|value| value.as_u64())
}
