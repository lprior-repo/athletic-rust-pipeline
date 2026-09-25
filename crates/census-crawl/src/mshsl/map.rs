use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;
use std::collections::HashSet;

use super::parse::{SchoolDetail, SchoolListRow};
use super::teams::{coach_role, is_published_level, team_sport, TeamCoaches};
use super::text::strip_honorific;
use super::{COACH_NAMESPACE, SOURCE_ID};

/// Normalise a role label: trim, drop a trailing colon, collapse inner whitespace, lowercase.
fn normalize_label(label: &str) -> String {
    label
        .trim()
        .trim_end_matches(':')
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

/// Map an Administration-block role onto an athletic-director role.
///
/// Exact match on purpose: the same block publishes office staff whose labels contain
/// "activities director" ("AD Administrative Assistant", "Activities Director Secretary"), and those
/// must never be emitted as the school's activities director.
pub fn ad_role(label: &str) -> Option<CoachRole> {
    match normalize_label(label).as_str() {
        "activities director"
        | "athletic director"
        | "assistant activities director"
        | "assistant athletic director"
        | "associate activities director"
        | "associate athletic director" => Some(CoachRole::AthleticDirector),
        _ => None,
    }
}

/// The provider's own key for a school: its numeric id when published, else the page slug.
pub fn provider_key(row: &SchoolListRow, detail: &SchoolDetail) -> String {
    detail.school_id.clone().unwrap_or_else(|| row.slug.clone())
}

/// Build the canonical school for one listing row plus its detail page.
///
/// The id is minted from state + normalized name, so the same school seen by another adapter resolves to
/// the same canonical id.
pub fn school_entities(
    row: &SchoolListRow,
    detail: &SchoolDetail,
    page_url: &str,
    observed_on: &str,
) -> Option<(CanonicalSchool, SchoolId)> {
    let name = if row.name.trim().is_empty() {
        detail.name.clone()?
    } else {
        row.name.clone()
    };
    let (mut school, school_id) =
        CanonicalSchool::new(UsJurisdiction::Minnesota, &name, normalize_name(&name));
    school.city = row.city.clone();
    school.association = Some(SOURCE_ID.to_string());
    school.enrollment = detail.enrollment;
    school.school_website = detail.website.clone();
    school.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: SOURCE_ID.to_string(),
            },
            provider_key(row, detail),
        )
        .with_url(page_url.to_string()),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new(SOURCE_ID, Some(page_url.to_string())),
        observed_on,
    ));
    Some((school, school_id))
}

/// Canonical athletic-director rows for one school page.
///
/// Only `Activities Director` / `Assistant Activities Director` entries are emitted; office roles are
/// dropped here, which is the last point before the store.
pub fn ad_coaches(
    detail: &SchoolDetail,
    school_id: &SchoolId,
    school_key: &str,
    page_url: &str,
    observed_on: &str,
) -> Vec<CanonicalCoach> {
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for entry in &detail.admin {
        if ad_role(&entry.role).is_none() {
            continue;
        }
        let name = strip_honorific(&entry.name);
        if name.is_empty() {
            continue;
        }
        let mut coach = CanonicalCoach::new(
            school_id,
            name.as_str(),
            None,
            Gender::Mixed,
            CoachRole::AthleticDirector,
        );
        if let Some(address) = entry.email() {
            coach.set_published_email(address);
        }
        coach.source_identities.push(
            SourceIdentity::new(
                SourceNamespace::AssociationSchool {
                    association: SOURCE_ID.to_string(),
                },
                format!("{school_key}:ad"),
            )
            .with_url(page_url.to_string()),
        );
        coach.evidence.push(Evidence::parsed(
            SourceRef::new(SOURCE_ID, Some(page_url.to_string())),
            observed_on,
        ));
        if seen.insert(coach.id.as_str().to_string()) {
            coaches.push(coach);
        }
    }
    coaches
}

/// The domains this school's own contacts are published on: its AD addresses plus its website host.
pub fn school_domains(detail: &SchoolDetail) -> Vec<String> {
    let mut domains: Vec<String> = Vec::new();
    for entry in &detail.admin {
        if ad_role(&entry.role).is_none() {
            continue;
        }
        for address in &entry.emails {
            if let Some(domain) = email_domain(address) {
                push_unique(&mut domains, domain);
            }
        }
    }
    if let Some(domain) = detail.website.as_deref().and_then(host_domain) {
        push_unique(&mut domains, domain);
    }
    domains
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn email_domain(address: &str) -> Option<String> {
    let (_, domain) = address.trim().rsplit_once('@')?;
    let domain = domain.trim().trim_matches('.').to_ascii_lowercase();
    (!domain.is_empty() && domain.contains('.')).then_some(domain)
}

fn host_domain(url: &str) -> Option<String> {
    let authority = url.split("://").nth(1).unwrap_or(url);
    let host = authority
        .split('/')
        .next()?
        .rsplit('@')
        .next()?
        .split(':')
        .next()?
        .trim_start_matches("www.")
        .to_ascii_lowercase();
    (!host.is_empty()).then_some(host)
}

/// Keep an API-published coach address unless it is malformed.
///
/// The `_domains` parameter remains part of the mapping helper's call shape, but publication is not
/// restricted to a school's own domains: `CanonicalCoach::set_published_email` classifies the
/// address's mailbox kind.
pub fn published_coach_email(address: &str, _domains: &[String]) -> Option<String> {
    let address = address.trim();
    email_domain(address)?;
    Some(address.to_string())
}

/// Canonical coach entities for the team coach payloads fetched for one school.
pub fn coach_entities(
    teams: &[TeamCoaches],
    school_id: &SchoolId,
    domains: &[String],
    observed_on: &str,
) -> Vec<CanonicalCoach> {
    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for team in teams {
        let Some((sport, gender)) = team_sport(&team.node.alias) else {
            continue;
        };
        for record in &team.records {
            if !is_published_level(&record.level) {
                continue;
            }
            let Some(role) = coach_role(&record.level) else {
                continue;
            };
            let name = strip_honorific(&record.name);
            if name.is_empty() {
                continue;
            }
            let mut coach =
                CanonicalCoach::new(school_id, name.as_str(), Some(sport), gender, role);
            if let Some(address) = record
                .email
                .as_deref()
                .and_then(|address| published_coach_email(address, domains))
            {
                coach.set_published_email(&address);
            }
            coach.source_identities.push(
                SourceIdentity::new(
                    SourceNamespace::Other(COACH_NAMESPACE.to_string()),
                    format!("{}:{}", team.node.nid, normalize_name(&name)),
                )
                .with_url(team.api_url.clone()),
            );
            coach.evidence.push(Evidence::parsed(
                SourceRef::new(SOURCE_ID, Some(team.api_url.clone())),
                observed_on,
            ));
            if let Some(page_url) = team.node.page_url() {
                coach.evidence.push(Evidence::parsed(
                    SourceRef::new(SOURCE_ID, Some(page_url)),
                    observed_on,
                ));
            }
            if seen.insert(coach.id.as_str().to_string()) {
                coaches.push(coach);
            }
        }
    }
    coaches
}
