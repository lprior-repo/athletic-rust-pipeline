use super::owner::{artifact, school_id};
use super::parse::{
    parse_gender, parse_school_profile, parse_sport_label, CoachRow, SchoolLink, SchoolProfile,
};
use super::{ASSOCIATION, SOURCE_ID, STATE};
use crate::net::FetchOutcome;
use crate::{
    directory::{postal_address, AddressParts},
    CrawlResult,
};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachContactClaim, CoachContactProgram,
    CoachRole, CoachTenure, CoachTenureEvidence, Evidence, SchoolId, SchoolPostalAddress,
    SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::school_directory::{PostalAddress, SourceLabel, StateRecordId};
use serde_json::json;

#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub school_id: SchoolId,
    pub coaches: Vec<CanonicalCoach>,
}

pub fn map_capture(indexed: &SchoolLink, capture: &FetchOutcome) -> CrawlResult<SchoolExtract> {
    let (id, profile) = qualified_profile(indexed, capture)?;
    let address = published_address(&profile.address)?;
    let (mut school, school_id) = CanonicalSchool::new(
        STATE,
        &profile.name,
        normalize_name(&profile.name),
        address
            .as_ref()
            .and_then(|value| value.city())
            .map(|city| city.as_str()),
    );
    let owner = SourceIdentity::new(
        SourceNamespace::association_school(ASSOCIATION),
        id.as_str(),
    )
    .with_url(capture.url.clone());
    school.source_identities.push(owner.clone());
    school.association = Some(ASSOCIATION.to_string());
    if !profile.classification.is_empty() {
        school.classification = Some(profile.classification);
    }
    let evidence = capture_evidence(capture);
    school.evidence.push(evidence.clone());
    if let Some(address) = address {
        let claim = SchoolPostalAddress::new(
            address,
            owner,
            SourceLabel::AthleticAssociation { state: STATE },
            evidence,
            capture.content_digest.clone(),
        )
        .map_err(|error| artifact(error.to_string()))?;
        school
            .add_postal_address(claim)
            .map_err(|error| artifact(error.to_string()))?;
    }
    let coaches: Vec<_> = profile
        .coaches
        .iter()
        .filter_map(|row| sport_coach(&school_id, id.as_str(), row, capture))
        .collect();
    coaches
        .iter()
        .flat_map(|coach| &coach.tenure_evidence)
        .try_for_each(|fact| {
            census_domain::model::validate_tenure_evidence(fact)
                .map_err(|error| artifact(error.to_string()))
        })?;
    Ok(SchoolExtract {
        school,
        school_id,
        coaches,
    })
}

fn qualified_profile(
    indexed: &SchoolLink,
    capture: &FetchOutcome,
) -> CrawlResult<(StateRecordId, SchoolProfile)> {
    let id = school_id(&indexed.url)?;
    if capture.status != 200
        || school_id(&capture.url)? != id
        || capture
            .response_url
            .as_deref()
            .map(school_id)
            .transpose()?
            .is_some_and(|final_id| final_id != id)
    {
        return Err(artifact(
            "UHSAA profile capture differs from its indexed school owner",
        ));
    }
    if capture.content_digest != crate::net::cache::content_digest(&capture.body) {
        return Err(artifact(
            "UHSAA profile capture digest differs from retained bytes",
        ));
    }
    let html = std::str::from_utf8(&capture.body).map_err(|error| artifact(error.to_string()))?;
    let published = super::published::read(html)?;
    let mut profile = parse_school_profile(html);
    profile.name = published.name;
    profile.address = published.address;
    let fetched = chrono::DateTime::parse_from_rfc3339(&capture.fetched_at)
        .map_err(|error| artifact(format!("UHSAA capture fetched_at: {error}")))?;
    if !(1900..=2100).contains(&chrono::Datelike::year(&fetched)) {
        return Err(artifact("UHSAA capture fetched_at is outside 1900–2100"));
    }
    if profile.name.is_empty() || normalize_name(&profile.name) != normalize_name(&indexed.name) {
        return Err(artifact(
            "UHSAA profile has no matching published school owner",
        ));
    }
    Ok((id, profile))
}

fn published_address(raw: &str) -> CrawlResult<Option<PostalAddress>> {
    if raw.is_empty() {
        return Ok(None);
    }
    let (street_city, state_zip) = raw
        .rsplit_once(',')
        .ok_or_else(|| artifact("UHSAA postal locality is missing"))?;
    let (street, city) = street_city
        .rsplit_once(',')
        .ok_or_else(|| artifact("UHSAA postal city is missing"))?;
    let (state, zip) = state_zip
        .trim()
        .split_once(' ')
        .ok_or_else(|| artifact("UHSAA postal state/ZIP is missing"))?;
    if census_domain::UsJurisdiction::parse(state) != Some(STATE) {
        return Err(artifact("UHSAA postal jurisdiction is not Utah"));
    }
    postal_address(AddressParts {
        street: street.trim(),
        city: city.trim(),
        state,
        zip: zip.trim(),
        ..AddressParts::default()
    })
    .map_err(|failure| artifact(failure.error.to_string()))
}

fn capture_evidence(capture: &FetchOutcome) -> Evidence {
    let mut evidence = Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(capture.url.clone())),
        &capture.fetched_at,
    );
    evidence.note = Some(json!({"capture_url": capture.url, "sha256": capture.content_digest, "fetched_at": capture.fetched_at}).to_string());
    evidence
}

fn sport_coach(
    school: &SchoolId,
    owner: &str,
    row: &CoachRow,
    capture: &FetchOutcome,
) -> Option<CanonicalCoach> {
    let sport = parse_sport_label(&row.sport_label)?;
    let gender = parse_gender(&row.sport_label);
    let mut coach =
        CanonicalCoach::new(school, &row.name, Some(sport), gender, CoachRole::HeadCoach);
    coach.set_published_email(&row.email);
    coach.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::association_school(ASSOCIATION),
            format!("{owner}:coach:{}", row.sport_label),
        )
        .with_url(capture.url.clone()),
    );
    coach.evidence.push(capture_evidence(capture));
    coach.tenure_evidence.push(CoachTenureEvidence {
        tenure: CoachTenure::Unknown,
        source: SourceRef::new(SOURCE_ID, Some(capture.url.clone())),
        source_sha256: capture.content_digest.clone(),
        retrieved_at: capture.fetched_at.clone(),
        statement: row.sport_label.clone(),
        claim: Some(CoachContactClaim {
            coach: coach.id.clone(),
            school: school.clone(),
            role: coach.role,
            program: CoachContactProgram::Team { sport, gender },
            mailbox: census_domain::model::published_email(&row.email).map(|(address, _)| address),
        }),
    });
    Some(coach)
}
