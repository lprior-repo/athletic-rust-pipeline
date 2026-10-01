use super::parse::{CoachRow, OrgSchool, PrimaryContact};
use super::{ASSOCIATION, SOURCE_ID};
use crate::coach_contacts::{parse_gender, parse_role, parse_sport};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;

pub fn map_org_school(
    row: &OrgSchool,
    state: UsJurisdiction,
    source_url: &str,
    observed_on: &str,
) -> Option<(CanonicalSchool, SchoolId)> {
    if row.name.is_empty() {
        return None;
    }
    let normalized = normalize_name(&row.name);
    let (mut school, id) = CanonicalSchool::new(state, &row.name, &normalized);
    school.association = Some(ASSOCIATION.to_string());
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(source_url.to_string())),
        observed_on,
    ));
    if let Some(public_id) = row.public_id {
        school.source_identities.push(
            SourceIdentity::new(
                SourceNamespace::association_school(SOURCE_ID),
                public_id.to_string(),
            )
            .with_url(source_url),
        );
    }
    if let Some(enrollment) = row.enrollment {
        school.enrollment = Some(enrollment);
    }
    Some((school, id))
}

pub fn map_primary_contact(
    contact: &PrimaryContact,
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> Option<CanonicalCoach> {
    let person = joined_name(&contact.first_name, &contact.last_name)?;
    let role = classify_role(&contact.role_name)?;
    let mut coach = CanonicalCoach::new(school_id, person, None, Gender::Mixed, role);
    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(source_url.to_string())),
        observed_on,
    ));
    Some(coach)
}

pub fn map_coach_row(
    row: &CoachRow,
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> Option<CanonicalCoach> {
    let person = joined_name(&row.first_name, &row.last_name)?;
    let role = classify_role(&row.position)?;
    if role == CoachRole::AthleticDirector {
        return None;
    }
    if !row.level.is_empty() && !row.level.to_ascii_lowercase().contains("varsity") {
        return None;
    }
    let (sport, gender) = classify_sport_gender(&row.sport, &row.level)?;
    let mut coach = CanonicalCoach::new(school_id, person, Some(sport), gender, role);
    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(source_url.to_string())),
        observed_on,
    ));
    Some(coach)
}

fn joined_name(first: &str, last: &str) -> Option<String> {
    let joined = match (first.is_empty(), last.is_empty()) {
        (true, true) => return None,
        (true, false) => last.to_string(),
        (false, true) => first.to_string(),
        (false, false) => format!("{first} {last}"),
    };
    Some(joined)
}

fn classify_sport_gender(sport_name: &str, level_name: &str) -> Option<(Sport, Gender)> {
    let lowered_sport = sport_name.to_ascii_lowercase();
    if lowered_sport.contains("ski") {
        return None;
    }
    let (sport, sport_gender) = parse_sport(sport_name)?;
    let gender = if sport_gender != Gender::Mixed {
        sport_gender
    } else {
        gender_from_label(level_name)
            .or_else(|| gender_from_label(sport_name))
            .unwrap_or(Gender::Mixed)
    };
    Some((sport, gender))
}

fn gender_from_label(label: &str) -> Option<Gender> {
    match parse_gender(label) {
        Gender::Unknown => {
            let lowered = label.to_ascii_lowercase();
            if lowered.contains("girls") || lowered.contains("women") {
                Some(Gender::Girls)
            } else if lowered.contains("boys") || lowered.contains("men") {
                Some(Gender::Boys)
            } else {
                None
            }
        }
        gender => Some(gender),
    }
}

fn classify_role(label: &str) -> Option<CoachRole> {
    match parse_role(label) {
        Some(CoachRole::Unknown) | None => None,
        role => role,
    }
}
