use super::Rules;
use crate::{CrawlError, CrawlResult};
use census_domain::model::{CoachRole, Gender, Sport};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct StaffContactHit {
    pub published_school: String,
    pub member_id: String,
    pub category_id: String,
    pub name: String,
    pub published_role: String,
    pub role: CoachRole,
    pub published_sport: String,
    pub sport: Option<Sport>,
    pub gender: Gender,
    pub published_level: String,
    pub email: String,
    pub url: String,
}

pub(super) fn parse(
    rules: &Rules,
    url: &str,
    html: &str,
) -> CrawlResult<Option<Vec<StaffContactHit>>> {
    if !crate::coach_directories::inspect_staff_links(html.as_bytes())?.0 {
        return Ok(None);
    }
    let directory = crate::sidearm_staff::parse_staff_directory(html)?;
    if directory.name.len() > 4096 || directory.members.len() > 320 {
        return Err(CrawlError::Schema {
            url: url.to_owned(),
            detail: "staff candidate directory exceeds name or row budget".to_owned(),
        });
    }
    let mut hits = Vec::new();
    for row in directory.members {
        if let Some(hit) = candidate(rules, url, &directory.name, row)? {
            hits.push(hit);
        }
    }
    Ok(Some(hits))
}

fn candidate(
    rules: &Rules,
    url: &str,
    school: &str,
    row: crate::sidearm_staff::StaffRow,
) -> CrawlResult<Option<StaffContactHit>> {
    if [
        &row.id,
        &row.category_id,
        &row.name,
        &row.role,
        &row.sport,
        &row.level,
        &row.email,
    ]
    .iter()
    .any(|value| value.len() > 4096)
    {
        return Err(CrawlError::Schema {
            url: url.to_owned(),
            detail: "staff candidate field exceeds 4096 bytes".to_owned(),
        });
    }
    let Some(role) = crate::coach_contacts::parse_role(&row.role) else {
        return Ok(None);
    };
    if rules.junk.is_match(&row.role) || row.id.is_empty() {
        return Ok(None);
    }
    let program = if role == CoachRole::AthleticDirector {
        (None, Gender::Unknown)
    } else if row.role.to_ascii_lowercase().contains("coach") {
        let Some((sport, gender)) = crate::coach_contacts::parse_sport(&row.sport) else {
            return Ok(None);
        };
        (Some(sport), gender)
    } else {
        return Ok(None);
    };
    let Some(name) = crate::row_hygiene::sanitize_person(&row.name)? else {
        return Ok(None);
    };
    let Some(email) = rules.sanitize_email(&row.email) else {
        return Ok(None);
    };
    Ok(Some(StaffContactHit {
        published_school: school.to_owned(),
        member_id: row.id,
        category_id: row.category_id,
        name,
        published_role: row.role,
        role,
        published_sport: row.sport,
        sport: program.0,
        gender: program.1,
        published_level: row.level,
        email,
        url: url.to_owned(),
    }))
}
