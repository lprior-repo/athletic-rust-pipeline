use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;

use super::parse::{MemberSchool, SchoolCoachRow};
use super::{ASSOCIATION, SOURCE_ID};

pub fn map_directory_row(
    row: &MemberSchool,
    source_url: &str,
    observed_on: &str,
) -> Option<(CanonicalSchool, SchoolId)> {
    let name = row.name.as_deref().map_or("", |value| value).trim();
    if name.is_empty() {
        return None;
    }
    let normalized = normalize_name(name);
    let (mut school, id) = CanonicalSchool::new(
        UsJurisdiction::Colorado,
        name,
        &normalized,
        row.city.as_deref().and_then(nonempty).as_deref(),
    );
    school.association = Some(ASSOCIATION.to_string());
    school.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::association_school(SOURCE_ID),
            row.school_code
                .map(|code| code.to_string())
                .as_deref()
                .map_or("", |value| value),
        )
        .with_url(source_url),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(source_url.to_string())),
        observed_on,
    ));
    Some((school, id))
}

pub fn map_coach_row(
    row: &SchoolCoachRow,
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> Option<CanonicalCoach> {
    if row.person.trim().is_empty() {
        return None;
    }
    let (sport, gender) = classify_activity(&row.activity_name);
    let sport = sport?;
    let role = classify_role(&row.title)?;
    if role == CoachRole::AthleticDirector {
        return None;
    }
    let name = row.person.trim().to_string();
    let mut coach = CanonicalCoach::new(school_id, &name, Some(sport), gender, role);
    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(source_url.to_string())),
        observed_on,
    ));
    Some(coach)
}

fn classify_activity(activity_name: &str) -> (Option<Sport>, Gender) {
    let lowered = activity_name.to_lowercase();
    let gender = if lowered.starts_with("boys") {
        Gender::Boys
    } else if lowered.starts_with("girls") {
        Gender::Girls
    } else {
        Gender::Mixed
    };
    if lowered.contains("track") {
        (Some(Sport::OutdoorTrack), gender)
    } else if lowered.contains("cross country") {
        (Some(Sport::CrossCountry), gender)
    } else {
        (None, gender)
    }
}

fn classify_role(title: &str) -> Option<CoachRole> {
    let lowered = title.to_lowercase();
    if lowered.contains("head coach") {
        Some(CoachRole::HeadCoach)
    } else if lowered.contains("assistant coach") {
        Some(CoachRole::AssistantCoach)
    } else if lowered.contains("athletic director") {
        Some(CoachRole::AthleticDirector)
    } else {
        None
    }
}

fn nonempty(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
