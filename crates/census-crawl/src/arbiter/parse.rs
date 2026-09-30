use crate::{CrawlError, CrawlResult};

pub struct Page<T> {
    pub total: u64,
    pub rows: Vec<T>,
}

pub struct OrgSchool {
    pub name: String,
    pub public_id: Option<u64>,
    pub org_id: Option<u64>,
    pub phone: Option<String>,
    pub enrollment: Option<u32>,
    pub primary_contact: Option<PrimaryContact>,
}

pub struct PrimaryContact {
    pub first_name: String,
    pub last_name: String,
    pub role_name: String,
}

pub struct CoachRow {
    pub first_name: String,
    pub last_name: String,
    pub position: String,
    pub sport: String,
    pub level: String,
}

pub fn credentials_in_bundle(bundle: &str) -> Option<(String, String)> {
    const ID_MARKER: &str = "client_id:\"";
    const SECRET_MARKER: &str = "\",client_secret:\"";
    let after_id = bundle.find(ID_MARKER)?.checked_add(ID_MARKER.len())?;
    let rest = bundle.get(after_id..)?;
    let before_secret = rest.find(SECRET_MARKER)?;
    let client_id = rest.get(..before_secret)?;
    let after_secret = rest.get(before_secret.checked_add(SECRET_MARKER.len())?..)?;
    let secret_end = after_secret.find('"')?;
    let client_secret = after_secret.get(..secret_end)?;
    if client_id.is_empty() || client_secret.is_empty() {
        return None;
    }
    Some((client_id.to_string(), client_secret.to_string()))
}

pub fn parse_token(body: &str, url: &str) -> CrawlResult<String> {
    let json = decode(body, url)?;
    json.get("access_token")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| CrawlError::Schema {
            url: url.to_string(),
            detail: "expected a non-empty access_token string".to_string(),
        })
}

pub fn parse_org_schools(body: &str, url: &str) -> CrawlResult<Page<OrgSchool>> {
    decode_page(body, url, "schools", decode_org_school)
}

pub fn parse_coach_rows(body: &str, url: &str) -> CrawlResult<Page<CoachRow>> {
    decode_page(body, url, "coaches", decode_coach_row)
}

fn decode_page<T>(
    body: &str,
    url: &str,
    subject: &str,
    map_row: impl Fn(&serde_json::Map<String, serde_json::Value>) -> T,
) -> CrawlResult<Page<T>> {
    let json = decode(body, url)?;
    let (total, rows) = page_rows(&json, url, subject)?;
    let mut decoded = Vec::with_capacity(rows.len());
    for row in rows {
        let object = row.as_object().ok_or_else(|| CrawlError::Schema {
            url: url.to_string(),
            detail: format!("expected an object in the {subject} rows"),
        })?;
        decoded.push(map_row(object));
    }
    Ok(Page {
        total,
        rows: decoded,
    })
}

fn decode(body: &str, url: &str) -> CrawlResult<serde_json::Value> {
    serde_json::from_str(body).map_err(|source| CrawlError::Decode {
        url: url.to_string(),
        source,
    })
}

fn page_rows<'a>(
    json: &'a serde_json::Value,
    url: &str,
    subject: &str,
) -> CrawlResult<(u64, &'a [serde_json::Value])> {
    let data = json
        .get("data")
        .and_then(|value| value.as_object())
        .ok_or_else(|| CrawlError::Schema {
            url: url.to_string(),
            detail: format!("expected an object at data for {subject}"),
        })?;
    let total = data
        .get("total")
        .and_then(|value| value.as_u64())
        .ok_or_else(|| CrawlError::Schema {
            url: url.to_string(),
            detail: format!("expected a u64 at data.total for {subject}"),
        })?;
    let rows = data
        .get("rows")
        .and_then(|value| value.as_array())
        .ok_or_else(|| CrawlError::Schema {
            url: url.to_string(),
            detail: format!("expected an array at data.rows for {subject}"),
        })?;
    Ok((total, rows.as_slice()))
}

fn decode_org_school(object: &serde_json::Map<String, serde_json::Value>) -> OrgSchool {
    let name = text(object, "name");
    let public_id = count(object, "publicId");
    let org_id = count(object, "id");
    let phone = nonempty(object, "phoneNumber");
    let enrollment = count(object, "enrollmentCount").and_then(|count| {
        let narrowed = u32::try_from(count).ok();
        narrowed.filter(|enrollment| *enrollment > 0)
    });
    let primary_contact = object
        .get("primaryContact")
        .and_then(|value| value.as_object())
        .map(decode_primary_contact);
    OrgSchool {
        name,
        public_id,
        org_id,
        phone,
        enrollment,
        primary_contact,
    }
}

fn decode_primary_contact(object: &serde_json::Map<String, serde_json::Value>) -> PrimaryContact {
    PrimaryContact {
        first_name: text(object, "firstName"),
        last_name: text(object, "lastName"),
        role_name: text(object, "roleName"),
    }
}

fn decode_coach_row(object: &serde_json::Map<String, serde_json::Value>) -> CoachRow {
    CoachRow {
        first_name: text(object, "firstName"),
        last_name: text(object, "lastName"),
        position: text(object, "coachPositionName"),
        sport: text(object, "sportName"),
        level: text(object, "levelName"),
    }
}

fn text(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    object
        .get(key)
        .and_then(|value| value.as_str())
        .map(clean)
        .unwrap_or_default()
}

fn nonempty(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    let value = text(object, key);
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn count(object: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<u64> {
    object
        .get(key)
        .and_then(|value| value.as_u64())
        .filter(|count| *count > 0)
}

fn clean(value: &str) -> String {
    let mut result = String::with_capacity(value.len());
    let mut pending_space = false;
    for character in value.chars() {
        if character.is_whitespace() {
            pending_space = !result.is_empty();
            continue;
        }
        if pending_space {
            result.push(' ');
            pending_space = false;
        }
        result.push(character);
    }
    result
}
