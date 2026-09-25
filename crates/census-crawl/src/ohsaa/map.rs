//! Canonical mapping: parsed rows -> canonical entities from `census_domain::model`.
//!
//! Nothing here reads the network, the store or a file: it takes the parsed rows from
//! [`super::parse`] and [`super::pages`] and returns the canonical types, which keeps it
//! unit-testable and reusable by the durable services.

use super::pages::{parse_ad_page, parse_sport_label, parse_sports_table};
use super::parse::{nonempty, sport_key, strip_honorific};
use super::{AD_PATH, ASSOCIATION, HOST, SCHOOL_INFO_PATH, SOURCE_ID, SPORTS_PATH, STATE};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};

// ── Parsed shapes ──────────────────────────────────────────────────────────

/// One row of the search result table.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchResult {
    /// ALL-CAPS school name as published (e.g. "DUBLIN COFFMAN").
    pub name: String,
    /// City from the second column (e.g. "Dublin").
    pub city: String,
    /// OHSAA numeric school id (e.g. "474").
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

/// One coach extracted from a sports-information cell.
#[derive(Debug, Clone)]
pub struct CoachEntry {
    pub name: String,
    pub email: Option<String>,
}

/// Parsed content from an athletic department page.
#[derive(Debug, Clone, Default)]
pub struct AdPage {
    pub director: Option<(String, Option<String>)>,
    pub office_roles: Vec<(String, String)>,
}

/// Canonical entities for one school.
#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub school_id: SchoolId,
    pub coaches: Vec<CanonicalCoach>,
}

// ── Entity mapping ─────────────────────────────────────────────────────────

/// Build canonical school and coach entities from a school's search result,
/// sports page, and AD page.
pub fn school_entities(
    result: &SearchResult,
    sports_html: &str,
    ad_html: &str,
    observed_on: &str,
) -> SchoolExtract {
    // Build school
    let (school, school_id) = school_from_result(result, observed_on);

    // Collect coaches
    let mut coaches: Vec<CanonicalCoach> = Vec::new();

    // Parse AD
    coaches.extend(ad_coach(
        parse_ad_page(ad_html).director,
        result,
        &school_id,
        observed_on,
    ));

    // Parse sports sections
    for (sport_label, boys, girls) in parse_sports_table(sports_html) {
        let Some(sport) = parse_sport_label(&sport_label) else {
            continue;
        };
        // Boys coach
        if let Some(entry) = boys {
            coaches.push(sport_coach(
                result,
                &school_id,
                sport,
                entry,
                Gender::Boys,
                "boys",
                observed_on,
            ));
        }

        // Girls coach
        if let Some(entry) = girls {
            coaches.push(sport_coach(
                result,
                &school_id,
                sport,
                entry,
                Gender::Girls,
                "girls",
                observed_on,
            ));
        }
    }

    SchoolExtract {
        school,
        school_id,
        coaches,
    }
}

/// Canonical school row for one search result, with its OHSAA identity and page evidence.
fn school_from_result(result: &SearchResult, observed_on: &str) -> (CanonicalSchool, SchoolId) {
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
        .with_url(page_url.clone()),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(page_url.clone())),
        observed_on.to_string(),
    ));
    (school, school_id)
}

/// The athletic director named on the AD page, if the page names one.
fn ad_coach(
    director: Option<(String, Option<String>)>,
    result: &SearchResult,
    school_id: &SchoolId,
    observed_on: &str,
) -> Option<CanonicalCoach> {
    let (ad_name, ad_email) = director?;
    let ad_url = result.ad_url();
    let mut coach = CanonicalCoach::new(
        school_id,
        strip_honorific(&ad_name),
        None, // AD is school-wide
        Gender::Mixed,
        CoachRole::AthleticDirector,
    );
    if let Some(email) = ad_email.as_deref() {
        coach.set_published_email(email);
    }
    coach.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: ASSOCIATION.to_string(),
            },
            format!("ad:{}", result.ohsaa_id),
        )
        .with_url(ad_url.clone()),
    );
    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(ad_url)),
        observed_on.to_string(),
    ));
    Some(coach)
}

/// One head coach from a sports-table cell: `gender` and `gender_key` say which side it is.
fn sport_coach(
    result: &SearchResult,
    school_id: &SchoolId,
    sport: Sport,
    entry: CoachEntry,
    gender: Gender,
    gender_key: &str,
    observed_on: &str,
) -> CanonicalCoach {
    let sports_url = result.sports_url();
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
    coach.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: ASSOCIATION.to_string(),
            },
            format!(
                "coach:{}:{}:{}",
                result.ohsaa_id,
                sport_key(&sport),
                gender_key
            ),
        )
        .with_url(sports_url.clone()),
    );
    coach.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(sports_url)),
        observed_on.to_string(),
    ));
    coach
}
