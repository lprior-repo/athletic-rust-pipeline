use super::parse::{nonempty, SchoolRecord, StaffPerson};
use super::staff::{parse_coach_title, parse_role, strip_honorific};
use super::ASSOCIATION;
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;

pub fn parse_school(
    record: &SchoolRecord,
    source_url: &str,
    observed_on: &str,
) -> Option<(CanonicalSchool, SchoolId)> {
    let name = record.name_formal.trim();
    if name.is_empty() {
        return None;
    }
    let normalized = normalize_name(name);
    let (mut school, id) = CanonicalSchool::new(
        UsJurisdiction::Illinois,
        name,
        &normalized,
        nonempty(&record.city).as_deref(),
    );
    school.association = Some(ASSOCIATION.to_string());
    school.school_website = record.url.as_ref().and_then(|v| nonempty(v));
    school.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: ASSOCIATION.to_string(),
            },
            &record.school_id,
        )
        .with_url(source_url),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new(ASSOCIATION, Some(source_url.to_string())),
        observed_on,
    ));
    Some((school, id))
}

pub fn parse_coach(
    person: &StaffPerson,
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> Option<CanonicalCoach> {
    let title = &person.default_title;
    let role = parse_role(title)?;

    let name = strip_honorific(&person.name);
    let mut coach = match role {
        CoachRole::AthleticDirector => {
            CanonicalCoach::new(school_id, &name, None, Gender::Mixed, role)
        }
        _ => match parse_coach_title(title) {
            Some((sport, gender)) => {
                CanonicalCoach::new(school_id, &name, Some(sport), gender, role)
            }
            None => CanonicalCoach::new(school_id, &name, None, Gender::Mixed, role),
        },
    };

    if let Some(address) = person.email.as_deref() {
        coach.set_published_email(address);
    }
    coach.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: ASSOCIATION.to_string(),
            },
            person.person_id.to_string(),
        )
        .with_url(source_url),
    );
    coach.evidence.push(Evidence::parsed(
        SourceRef::new(ASSOCIATION, Some(source_url.to_string())),
        observed_on,
    ));
    Some(coach)
}
