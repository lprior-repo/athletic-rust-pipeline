//! Row to canonical entities: the school one row describes, its sport coach, its school-wide
//! athletic director, and the athletic office its AD columns describe.

use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceRef, Sport,
};

use crate::sources::{CrawlError, CrawlResult};

use super::parse::{clean, nonempty, parse_role, parse_sport, strip_honorific};
use super::wire::{CoachContactRow, RowEntities, RowSource};

/// Build the canonical entities for one CSV row.
pub fn row_entities(row: &CoachContactRow, default_observed_on: &str) -> CrawlResult<RowEntities> {
    let state = clean(&row.state).to_ascii_uppercase();
    let school_name = clean(&row.school);
    let (mut school, school_id) = school_with_city(&state, &school_name, &row.city);
    let source = RowSource::of(row, default_observed_on, &school_name);
    school.source_identities.push(
        SourceIdentity::new(source.namespace.clone(), source.key.clone())
            .with_url(source.url.clone().unwrap_or_default()),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new("coach_contacts_csv", source.url.clone()),
        source.observed_on.clone(),
    ));

    let role = parse_role(&format!("{} {}", row.role, row.sport));
    let mut coaches = Vec::new();

    if let Some(coach) = primary_coach(row, &school_id, &source, role)? {
        coaches.push(coach);
    }

    // Coaching rows in IA/IL/NE/OH/WI also carry the school's AD columns; import that person so the
    // contact graph has an athletic office even where no dedicated AD row was captured. Rows whose
    // role is a non-athletic office (Superintendent/Principal/Trainer/Secretary) are dropped above and
    // never reach here, so their names cannot leak through this branch.
    if matches!(
        role,
        Some(CoachRole::HeadCoach | CoachRole::AssistantCoach | CoachRole::Unknown)
    ) {
        if let Some(coach) = imported_ad(row, &school_id, &source) {
            coaches.push(coach);
        }
    }

    Ok(RowEntities { school, coaches })
}

/// The canonical school one CSV row describes, with the city alias the row publishes.
fn school_with_city(state: &str, school_name: &str, city: &str) -> (CanonicalSchool, SchoolId) {
    let (mut school, school_id) =
        CanonicalSchool::new(state, school_name, normalize_name(school_name));
    school.city = nonempty(city);
    if let Some(city) = school.city.clone() {
        school.aliases.push(format!("{city} {state}"));
    }
    (school, school_id)
}

/// The sport a row's label resolves to (`None` when it names none) and the gender side it covers.
fn sport_of(row: &CoachContactRow) -> (Option<Sport>, Gender) {
    let sport_gender = parse_sport(&row.sport);
    let (sport, gender) = sport_gender.unwrap_or((Sport::OutdoorTrack, Gender::Mixed));
    (sport_gender.map(|_| sport), gender)
}

/// The role a sport-scoped coaching row must name; the message is the one the builder raised.
fn required_role(role: Option<CoachRole>) -> CrawlResult<CoachRole> {
    role.ok_or_else(|| CrawlError::Invariant {
        detail: "coaching role".to_string(),
    })
}

/// Set a coach row's email and attach its provider identity and evidence.
fn attach_source(
    coach: &mut CanonicalCoach,
    identity: String,
    email: Option<String>,
    source: &RowSource,
) {
    coach.professional_email = email;
    coach.source_identities.push(
        SourceIdentity::new(source.namespace.clone(), identity)
            .with_url(source.url.clone().unwrap_or_default()),
    );
    coach.evidence.push(Evidence::parsed(
        source.source_ref.clone(),
        source.observed_on.clone(),
    ));
}

/// The primary coach entity one CSV row describes: a sport coach, a school-wide AD, or nothing.
fn primary_coach(
    row: &CoachContactRow,
    school_id: &SchoolId,
    source: &RowSource,
    role: Option<CoachRole>,
) -> CrawlResult<Option<CanonicalCoach>> {
    match role {
        // A sport-scoped coaching row.
        Some(CoachRole::HeadCoach | CoachRole::AssistantCoach | CoachRole::Unknown) => {
            sport_coach(row, school_id, source, role)
        }
        // A school-wide athletic-director row.
        Some(CoachRole::AthleticDirector) => Ok(ad_coach(row, school_id, source)),
        None => Ok(None),
    }
}

/// The sport-scoped coaching entity one row describes, when the row names a coach.
fn sport_coach(
    row: &CoachContactRow,
    school_id: &SchoolId,
    source: &RowSource,
    role: Option<CoachRole>,
) -> CrawlResult<Option<CanonicalCoach>> {
    let Some(CoachRole::HeadCoach | CoachRole::AssistantCoach | CoachRole::Unknown) = role else {
        return Ok(None);
    };
    let Some(coach_name) = nonempty(&row.coach_name) else {
        return Ok(None);
    };
    let (sport, gender) = sport_of(row);
    let identity = format!("{}:{role:?}", source.key);
    let email = nonempty(&row.public_professional_email);
    let mut coach = CanonicalCoach::new(
        school_id,
        strip_honorific(&coach_name),
        sport,
        gender,
        required_role(role)?,
    );
    attach_source(&mut coach, identity, email, source);
    Ok(Some(coach))
}

/// The school-wide athletic-director entity an AD row describes.
fn ad_coach(
    row: &CoachContactRow,
    school_id: &SchoolId,
    source: &RowSource,
) -> Option<CanonicalCoach> {
    let ad_name = nonempty(&row.ad_name).or_else(|| nonempty(&row.coach_name))?;
    let email = nonempty(&row.ad_email).or_else(|| nonempty(&row.public_professional_email));
    Some(ad_entity(
        school_id,
        strip_honorific(&ad_name),
        email,
        source,
    ))
}

/// The athletic office a coaching row's AD columns describe.
fn imported_ad(
    row: &CoachContactRow,
    school_id: &SchoolId,
    source: &RowSource,
) -> Option<CanonicalCoach> {
    let ad_name = nonempty(&row.ad_name)?;
    Some(ad_entity(
        school_id,
        strip_honorific(&ad_name),
        nonempty(&row.ad_email),
        source,
    ))
}

/// Build the athletic-director coach entity both AD branches share.
fn ad_entity(
    school_id: &SchoolId,
    name: String,
    email: Option<String>,
    source: &RowSource,
) -> CanonicalCoach {
    let mut coach = CanonicalCoach::new(
        school_id,
        name,
        None,
        Gender::Mixed,
        CoachRole::AthleticDirector,
    );
    attach_source(&mut coach, format!("{}:ad", source.key), email, source);
    coach
}
