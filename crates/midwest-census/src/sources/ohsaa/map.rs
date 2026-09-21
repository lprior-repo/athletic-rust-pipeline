//! Canonical mapping: parsed rows -> canonical entities from `crate::model`.
//!
//! Nothing here reads the network, the store or a file: it takes the parsed rows from
//! [`super::parse`] and [`super::pages`] and returns the canonical types, which keeps it
//! unit-testable and reusable by the durable services.

use super::pages::{parse_ad_page, parse_sport_label, parse_sports_table};
use super::parse::{nonempty, sport_key, strip_honorific};
use super::{AD_PATH, ASSOCIATION, HOST, SCHOOL_INFO_PATH, SOURCE_ID, SPORTS_PATH, STATE};
use crate::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef,
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
    let city = nonempty(&result.city);
    let page_url = result.page_url();
    let sports_url = result.sports_url();
    let ad_url = result.ad_url();

    // Build school
    let (mut school, school_id) =
        CanonicalSchool::new(STATE, &result.name, normalize_name(&result.name));
    school.city = city;
    school.association = Some(ASSOCIATION.to_string());
    school.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: "ohsaa".into(),
            },
            &result.ohsaa_id,
        )
        .with_url(page_url.clone()),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(page_url.clone())),
        observed_on.to_string(),
    ));

    // Collect coaches
    let mut coaches: Vec<CanonicalCoach> = Vec::new();

    // Parse AD
    let ad = parse_ad_page(ad_html);
    if let Some((ad_name, ad_email)) = ad.director {
        let clean_name = strip_honorific(&ad_name);
        let mut coach = CanonicalCoach::new(
            &school_id,
            clean_name,
            None, // AD is school-wide
            Gender::Mixed,
            CoachRole::AthleticDirector,
        );
        coach.professional_email = ad_email;
        coach.source_identities.push(
            SourceIdentity::new(
                SourceNamespace::AssociationSchool {
                    association: "ohsaa".into(),
                },
                format!("ad:{}", result.ohsaa_id),
            )
            .with_url(ad_url.clone()),
        );
        coach.evidence.push(Evidence::parsed(
            SourceRef::new(SOURCE_ID, Some(ad_url)),
            observed_on.to_string(),
        ));
        coaches.push(coach);
    }

    // Parse sports sections
    let sections = parse_sports_table(sports_html);

    for (sport_label, boys, girls) in sections {
        if let Some(sport) = parse_sport_label(&sport_label) {
            // Boys coach
            if let Some(boys_entry) = boys {
                let clean_name = strip_honorific(&boys_entry.name);
                let mut coach = CanonicalCoach::new(
                    &school_id,
                    clean_name,
                    Some(sport),
                    Gender::Boys,
                    CoachRole::HeadCoach,
                );
                coach.professional_email = boys_entry.email;
                coach.source_identities.push(
                    SourceIdentity::new(
                        SourceNamespace::AssociationSchool {
                            association: "ohsaa".into(),
                        },
                        format!("coach:{}:{}:boys", result.ohsaa_id, sport_key(&sport)),
                    )
                    .with_url(sports_url.clone()),
                );
                coach.evidence.push(Evidence::parsed(
                    SourceRef::new(SOURCE_ID, Some(sports_url.clone())),
                    observed_on.to_string(),
                ));
                coaches.push(coach);
            }

            // Girls coach
            if let Some(girls_entry) = girls {
                let clean_name = strip_honorific(&girls_entry.name);
                let mut coach = CanonicalCoach::new(
                    &school_id,
                    clean_name,
                    Some(sport),
                    Gender::Girls,
                    CoachRole::HeadCoach,
                );
                coach.professional_email = girls_entry.email;
                coach.source_identities.push(
                    SourceIdentity::new(
                        SourceNamespace::AssociationSchool {
                            association: "ohsaa".into(),
                        },
                        format!("coach:{}:{}:girls", result.ohsaa_id, sport_key(&sport)),
                    )
                    .with_url(sports_url.clone()),
                );
                coach.evidence.push(Evidence::parsed(
                    SourceRef::new(SOURCE_ID, Some(sports_url.clone())),
                    observed_on.to_string(),
                ));
                coaches.push(coach);
            }
        }
    }

    SchoolExtract {
        school,
        school_id,
        coaches,
    }
}
