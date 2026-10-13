use super::parse::{parse_gender, parse_sport_label, ParsedRow, SchoolProfile};
use super::{ASSOCIATION, HOST, SOURCE_ID, STATE};
use crate::{CrawlError, CrawlResult};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, SchoolId, SourceIdentity,
    SourceNamespace, SourceRef,
};

#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub school_id: SchoolId,
    pub coaches: Vec<CanonicalCoach>,
}

pub fn school_entities(
    school_name: &str,
    aia_id: &str,
    city: Option<&str>,
    profile: &SchoolProfile,
    observed_on: &str,
) -> SchoolExtract {
    let normalized = normalize_name(school_name);
    let (mut school, school_id) = CanonicalSchool::new(
        STATE,
        school_name,
        &normalized,
        city.map(str::trim).filter(|value| !value.is_empty()),
    );
    school.association = Some(ASSOCIATION.to_string());

    let school_url = format!("{HOST}/schools/{aia_id}");
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(school_url.clone())),
        observed_on.to_string(),
    ));

    let mut coaches = Vec::new();
    for row in &profile.coaches {
        if let Some(sport) = parse_sport_label(&row.sport_label) {
            let gender = parse_gender(&row.sport_label);
            if let Some(coach) = sport_coach(
                &school_id,
                &row.sport_label,
                &row.name,
                sport,
                gender,
                &school_url,
                observed_on,
            ) {
                coaches.push(coach);
            }
        }
    }

    SchoolExtract {
        school,
        school_id,
        coaches,
    }
}

fn sport_coach(
    school_id: &SchoolId,
    sport_label: &str,
    coach_name: &str,
    sport: census_domain::model::Sport,
    gender: census_domain::model::Gender,
    url: &str,
    observed_on: &str,
) -> Option<CanonicalCoach> {
    if coach_name.is_empty() {
        return None;
    }

    let mut coach = CanonicalCoach::new(
        school_id,
        coach_name.to_string(),
        Some(sport),
        gender,
        CoachRole::HeadCoach,
    );

    coach.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: ASSOCIATION.to_string(),
            },
            format!("coach:{}:{}", sport_label, coach_name),
        )
        .with_url(url.to_string()),
    );

    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(url.to_string())),
        observed_on.to_string(),
    ));

    Some(coach)
}

pub fn qualify_owner(
    indexed: &ParsedRow,
    profile: &SchoolProfile,
    capture: &crate::net::FetchOutcome,
) -> CrawlResult<()> {
    let expected = indexed.school_id.trim();
    if expected.is_empty() || !expected.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(CrawlError::Schema {
            url: capture.url.clone(),
            detail: "AIA profile capture differs from its indexed school owner".to_string(),
        });
    }
    let seen = profile_id(&capture.url)?;
    if seen != expected {
        return Err(CrawlError::Schema {
            url: capture.url.clone(),
            detail: "AIA profile capture differs from its indexed school owner".to_string(),
        });
    }
    if let Some(response_url) = capture.response_url.as_deref() {
        let landed = profile_id(response_url)?;
        if landed != expected {
            return Err(CrawlError::Schema {
                url: capture.url.clone(),
                detail: "AIA profile redirect differs from its indexed school owner".to_string(),
            });
        }
    }
    if capture.status != 200 {
        return Err(CrawlError::Schema {
            url: capture.url.clone(),
            detail: "AIA profile capture differs from its indexed school owner".to_string(),
        });
    }
    if !owner_matches(indexed, profile) {
        return Err(CrawlError::Schema {
            url: capture.url.clone(),
            detail: "AIA profile has no matching published school owner".to_string(),
        });
    }
    Ok(())
}

fn owner_matches(indexed: &ParsedRow, profile: &SchoolProfile) -> bool {
    let published = normalize_name(&profile.info.name);
    if published.is_empty() {
        return false;
    }
    if normalize_name(&indexed.name) == published {
        return true;
    }
    indexed
        .full_name
        .as_deref()
        .is_some_and(|full| normalize_name(full) == published)
}

fn profile_id(raw: &str) -> CrawlResult<String> {
    let url = url::Url::parse(raw).map_err(|error| CrawlError::Schema {
        url: raw.to_string(),
        detail: error.to_string(),
    })?;
    if url.scheme() != "https" {
        return Err(CrawlError::Schema {
            url: raw.to_string(),
            detail: "AIA profile capture differs from its indexed school owner".to_string(),
        });
    }
    if url.host_str() != Some("aiaonline.org") {
        return Err(CrawlError::Schema {
            url: raw.to_string(),
            detail: "AIA profile capture differs from its indexed school owner".to_string(),
        });
    }
    if !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.fragment().is_some()
        || url.query().is_some()
    {
        return Err(CrawlError::Schema {
            url: raw.to_string(),
            detail: "AIA profile capture differs from its indexed school owner".to_string(),
        });
    }
    let Some(id) = url.path().strip_prefix("/schools/") else {
        return Err(CrawlError::Schema {
            url: raw.to_string(),
            detail: "AIA profile capture differs from its indexed school owner".to_string(),
        });
    };
    if id.is_empty() || id.contains('/') || !id.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(CrawlError::Schema {
            url: raw.to_string(),
            detail: "AIA profile capture differs from its indexed school owner".to_string(),
        });
    }
    let numeric = id.parse::<u64>().map_err(|error| CrawlError::Schema {
        url: raw.to_string(),
        detail: error.to_string(),
    })?;
    if numeric == 0 || numeric.to_string() != id {
        return Err(CrawlError::Schema {
            url: raw.to_string(),
            detail: "AIA profile capture differs from its indexed school owner".to_string(),
        });
    }
    Ok(id.to_string())
}
