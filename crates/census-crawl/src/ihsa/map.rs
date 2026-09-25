//! Canonical mapping: one IHSA school record or staff row becomes a canonical entity. The collector
//! reveals an address for every retained coach or athletic-director row whose staff payload advertises
//! `HasEmail`.

use super::parse::{nonempty, SchoolRecord, StaffPerson};
use super::staff::{parse_coach_title, parse_role, strip_honorific};
use super::ASSOCIATION;
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;

/// Convert one IHSA school record into a canonical school.
///
/// Returns `None` when the school name is empty.
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
    let (mut school, id) = CanonicalSchool::new(UsJurisdiction::Illinois, name, &normalized);
    school.city = nonempty(&record.city);
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

/// Convert one IHSA staff person into a canonical coach, if the title is a coaching or AD role.
///
/// Returns `None` for office roles (secretary, trainer, principal, etc.) and for staff whose
/// `DefaultTitle` does not map to a coaching or AD role.
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
        CoachRole::AthleticDirector => CanonicalCoach::new(
            school_id,
            &name,
            None,          // school-wide role
            Gender::Mixed, // AD is not gender-specific
            role,
        ),
        _ => {
            // Coaching role. Titles outside track & field / cross country ("Boys Bowling Head
            // Coach") are still kept as coaches of the school, but they carry no sport: guessing
            // outdoor track here would present a basketball coach as a track coach.
            match parse_coach_title(title) {
                Some((sport, gender)) => {
                    CanonicalCoach::new(school_id, &name, Some(sport), gender, role)
                }
                None => CanonicalCoach::new(school_id, &name, None, Gender::Mixed, role),
            }
        }
    };

    // `staff2` publishes the person, not the address — the collector fills the contact from the
    // reveal endpoint when `HasEmail` is set.
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
