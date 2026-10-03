use super::pages::{parse_ad_page, parse_sport_label, parse_sports_table, published_absence};
use super::parse::{nonempty, strip_honorific};
use super::{AD_PATH, ASSOCIATION, HOST, SCHOOL_INFO_PATH, SOURCE_ID, SPORTS_PATH, STATE};
use crate::net::FetchOutcome;
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchResult {
    pub name: String,
    pub city: String,
    pub ohsaa_id: String,
}

impl SearchResult {
    pub fn page_url(&self) -> String {
        format!("{HOST}{SCHOOL_INFO_PATH}?ohsaaId={}", self.ohsaa_id)
    }
    pub fn sports_url(&self) -> String {
        format!("{HOST}{SPORTS_PATH}?ohsaaId={}", self.ohsaa_id)
    }
    pub fn ad_url(&self) -> String {
        format!("{HOST}{AD_PATH}?ohsaaId={}", self.ohsaa_id)
    }
}

#[derive(Debug, Clone)]
pub struct CoachEntry {
    pub name: String,
    pub email: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AdPage {
    pub director: Option<(String, Option<String>)>,
    pub office_roles: Vec<(String, String)>,
}

#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub school_id: SchoolId,
    pub coaches: Vec<CanonicalCoach>,
}

pub fn school_entities(
    result: &SearchResult,
    sports: &FetchOutcome,
    ad: Option<&FetchOutcome>,
) -> SchoolExtract {
    let (school, school_id) = school_from_result(result, sports);
    let coaches = ad
        .and_then(|capture| {
            ad_coach(
                parse_ad_page(&String::from_utf8_lossy(&capture.body)).director,
                &school_id,
                capture,
            )
        })
        .into_iter()
        .chain(sports_coaches(&school_id, sports))
        .collect();
    SchoolExtract {
        school,
        school_id,
        coaches,
    }
}

fn sports_coaches<'a>(
    school_id: &'a SchoolId,
    capture: &'a FetchOutcome,
) -> impl Iterator<Item = CanonicalCoach> + 'a {
    parse_sports_table(&String::from_utf8_lossy(&capture.body))
        .into_iter()
        .flat_map(move |(label, boys, girls)| {
            let sport = parse_sport_label(&label);
            [(boys, Gender::Boys), (girls, Gender::Girls)]
                .into_iter()
                .filter_map(move |(entry, gender)| {
                    Some(sport_coach(school_id, sport?, entry?, gender, capture))
                })
        })
}

fn school_from_result(
    result: &SearchResult,
    capture: &FetchOutcome,
) -> (CanonicalSchool, SchoolId) {
    let page_url = result.page_url();
    let (mut school, school_id) =
        CanonicalSchool::new(STATE, &result.name, normalize_name(&result.name));
    school.city = nonempty(&result.city);
    school.association = Some(ASSOCIATION.to_string());
    school.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: ASSOCIATION.to_string(),
            },
            &result.ohsaa_id,
        )
        .with_url(page_url),
    );
    school.evidence.push(capture_evidence(capture));
    (school, school_id)
}

fn ad_coach(
    director: Option<(String, Option<String>)>,
    school_id: &SchoolId,
    capture: &FetchOutcome,
) -> Option<CanonicalCoach> {
    let (ad_name, ad_email) = director?;
    if published_absence(&ad_name) {
        return None;
    }
    let mut coach = CanonicalCoach::new(
        school_id,
        strip_honorific(&ad_name),
        None,
        Gender::Mixed,
        CoachRole::AthleticDirector,
    );
    if let Some(email) = ad_email.as_deref() {
        coach.set_published_email(email);
    }
    coach.evidence.push(capture_evidence(capture));
    Some(coach)
}

fn sport_coach(
    school_id: &SchoolId,
    sport: Sport,
    entry: CoachEntry,
    gender: Gender,
    capture: &FetchOutcome,
) -> CanonicalCoach {
    let mut coach = CanonicalCoach::new(
        school_id,
        strip_honorific(&entry.name),
        Some(sport),
        gender,
        CoachRole::HeadCoach,
    );
    if let Some(email) = entry.email.as_deref() {
        coach.set_published_email(email);
    }
    coach.evidence.push(capture_evidence(capture));
    coach
}

pub(super) fn capture_note(capture: &FetchOutcome) -> serde_json::Value {
    serde_json::json!({
        "capture_url": capture.url,
        "sha256": capture.content_digest,
        "acquired_at": capture.fetched_at,
    })
}

fn capture_evidence(capture: &FetchOutcome) -> Evidence {
    let mut evidence = Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(capture.url.clone())),
        &capture.fetched_at,
    );
    evidence.note = Some(capture_note(capture).to_string());
    evidence
}
