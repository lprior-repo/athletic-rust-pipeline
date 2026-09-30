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

fn decode_string(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    obj.get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn decode_u64(obj: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<u64> {
    obj.get(key).and_then(|v| v.as_u64())
}

pub fn decode_school_record(
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

pub fn parse_school_json(json_str: &str) -> CrawlResult<Vec<MemberSchool>> {
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
