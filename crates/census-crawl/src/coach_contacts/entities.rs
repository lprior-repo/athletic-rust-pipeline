use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceRef, Sport,
};
use census_domain::UsJurisdiction;

use crate::{CrawlError, CrawlResult};

use super::parse::{clean, nonempty, parse_role, parse_sport, strip_honorific};
use super::wire::{CoachContactRow, RowEntities, RowSource};

pub fn row_entities(
    row: &CoachContactRow,
    state: UsJurisdiction,
    default_observed_on: &str,
) -> CrawlResult<RowEntities> {
    let school_name = clean(&row.school);
    let (mut school, school_id) = school_with_city(state, &school_name, &row.city);
    let source = RowSource::of(row, default_observed_on, &school_name);
    school.source_identities.push(
        SourceIdentity::new(source.namespace.clone(), source.key.clone()).with_url(
            source
                .url
                .clone()
                .map_or(Default::default(), core::convert::identity),
        ),
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

fn school_with_city(
    state: UsJurisdiction,
    school_name: &str,
    city: &str,
) -> (CanonicalSchool, SchoolId) {
    let (mut school, school_id) = CanonicalSchool::new(
        state,
        school_name,
        normalize_name(school_name),
        nonempty(city).as_deref(),
    );
    if let Some(city) = school.city.clone() {
        school.aliases.push(format!("{city} {}", state.code()));
    }
    (school, school_id)
}

fn sport_of(row: &CoachContactRow) -> (Option<Sport>, Gender) {
    let sport_gender = parse_sport(&row.sport);
    let (sport, gender) = sport_gender.map_or((Sport::OutdoorTrack, Gender::Mixed), |value| value);
    (sport_gender.map(|_| sport), gender)
}

fn required_role(role: Option<CoachRole>) -> CrawlResult<CoachRole> {
    role.ok_or_else(|| CrawlError::Invariant {
        detail: "coaching role".to_string(),
    })
}

fn attach_source(
    coach: &mut CanonicalCoach,
    identity: String,
    email: Option<String>,
    source: &RowSource,
) {
    if let Some(email) = email.as_deref() {
        coach.set_published_email(email);
    }
    coach.source_identities.push(
        SourceIdentity::new(source.namespace.clone(), identity).with_url(
            source
                .url
                .clone()
                .map_or(Default::default(), core::convert::identity),
        ),
    );
    coach.evidence.push(Evidence::parsed(
        source.source_ref.clone(),
        source.observed_on.clone(),
    ));
}

fn primary_coach(
    row: &CoachContactRow,
    school_id: &SchoolId,
    source: &RowSource,
    role: Option<CoachRole>,
) -> CrawlResult<Option<CanonicalCoach>> {
    match role {
        Some(CoachRole::HeadCoach | CoachRole::AssistantCoach | CoachRole::Unknown) => {
            sport_coach(row, school_id, source, role)
        }
        Some(CoachRole::AthleticDirector) => Ok(ad_coach(row, school_id, source)),
        None => Ok(None),
    }
}

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
