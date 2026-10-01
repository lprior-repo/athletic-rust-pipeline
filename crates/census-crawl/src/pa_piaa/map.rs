use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;

use super::parse::{ContactRow, SchoolRow};
use super::{nonempty, ASSOCIATION, SOURCE_ID};

pub fn map_directory_row(
    row: &SchoolRow,
    source_url: &str,
    observed_on: &str,
) -> Option<(CanonicalSchool, SchoolId)> {
    let name = row.name.trim();
    if name.is_empty() {
        return None;
    }
    let normalized = normalize_name(name);
    let (mut school, id) = CanonicalSchool::new(UsJurisdiction::Pennsylvania, name, &normalized);
    school.city = row.city.as_deref().and_then(nonempty);
    school.association = Some(ASSOCIATION.to_string());
    school.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::association_school(SOURCE_ID),
            row.school_id.as_str(),
        )
        .with_url(row.detail_url.clone()),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(source_url.to_string())),
        observed_on,
    ));
    Some((school, id))
}
pub fn map_contact_row(
    row: &ContactRow,
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> Option<CanonicalCoach> {
    let person = row.person.trim();
    if person.is_empty() {
        return None;
    }
    let mut coach = CanonicalCoach::new(
        school_id,
        person,
        None,
        Gender::Mixed,
        CoachRole::AthleticDirector,
    );
    if let Some(address) = row.email.as_deref().and_then(nonempty) {
        coach.set_published_email(address.as_str());
    }
    coach.source_identities.push(
        SourceIdentity::new(SourceNamespace::association_school(SOURCE_ID), person)
            .with_url(source_url.to_string()),
    );
    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(source_url.to_string())),
        observed_on,
    ));
    Some(coach)
}
