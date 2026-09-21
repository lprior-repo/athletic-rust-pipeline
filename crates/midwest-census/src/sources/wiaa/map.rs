//! Canonical mapping: parsed WIAA rows -> schools, athletic directors and coaches.
//!
//! Nothing here reads the network, the store or a file: it takes the row shapes from `parse`
//! and returns the canonical types from `crate::model`.

use crate::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SourceIdentity,
    SourceNamespace, SourceRef, Sport,
};
use std::collections::HashSet;

use super::parse::{IndexEntry, SchoolPage};
use super::primitives::{clean, meaningful, valid_email};
use super::{ASSOCIATION, SOURCE_ID};

/// Canonical entities for one school, plus a transcript of what the page offered and the model did
/// not take.
#[derive(Debug, Clone)]
pub struct SchoolExtract {
    pub school: CanonicalSchool,
    pub coaches: Vec<CanonicalCoach>,
    /// Distinct non-director administration roles seen on the page (superintendent, principal,
    /// secretary, AD admin assistant, …). Never emitted as a coach or an athletic director.
    pub skipped_admin_roles: Vec<String>,
    /// Coach-table rows dropped because the sport is not TF/XC, the role is not a coaching role, or
    /// the row carries no person name.
    pub skipped_coach_rows: usize,
}

// -------------------------------------------------------------------------------------------------
// Label mapping
// -------------------------------------------------------------------------------------------------

/// Map a published sport label onto the sport ontology plus the gender side it covers.
///
/// `Boys Track and Field` → outdoor track, boys; `Girls Cross Country` → cross country, girls;
/// `Coed …` → mixed. Any other sport returns `None`, which is what keeps this adapter to TF/XC.
pub fn parse_sport_label(label: &str) -> Option<(Sport, Gender)> {
    let lowered = label.trim().to_ascii_lowercase();
    let sport = if lowered.contains("cross country") || lowered.contains("cross-country") {
        Sport::CrossCountry
    } else if lowered.contains("indoor") {
        Sport::IndoorTrack
    } else if lowered.contains("track") {
        // WIAA sanctions no indoor season: every track row on this surface is outdoor.
        Sport::OutdoorTrack
    } else {
        return None;
    };
    let gender = if lowered.contains("girls") || lowered.contains("women") {
        Gender::Girls
    } else if lowered.contains("boys") || lowered.contains("men") {
        Gender::Boys
    } else if lowered.contains("coed") || lowered.contains("co-ed") {
        Gender::Mixed
    } else {
        Gender::Unknown
    };
    Some((sport, gender))
}

/// Map a `#tblCoachList` role label onto the coach-role vocabulary.
///
/// Only published coaching roles are accepted; anything without the word "coach" is skipped.
pub fn parse_coach_role(label: &str) -> Option<CoachRole> {
    let lowered = label.trim().to_ascii_lowercase();
    if !lowered.contains("coach") {
        return None;
    }
    if lowered.contains("assistant") || lowered.contains("asst") {
        return Some(CoachRole::AssistantCoach);
    }
    if lowered.contains("head") {
        return Some(CoachRole::HeadCoach);
    }
    Some(CoachRole::Unknown)
}

/// Roles that mention an athletic director without being the school's athletic director: WIAA
/// publishes office staff ("AD Admin Assistant") in the same administration table as the AD, and its
/// assistant/associate AD rows are a different person from the director.
const NON_DIRECTOR_TOKENS: [&str; 10] = [
    "secretary",
    "administrative assistant",
    "admin assistant",
    "assistant",
    "asst",
    "associate",
    "trainer",
    "principal",
    "superintendent",
    "business manager",
];

/// Map a `#tblAdminList` role label onto a canonical role.
///
/// `Some(AthleticDirector)` only for the athletic/activities director. Office, medical, building and
/// assistant-administration roles return `None`, so they can never be imported as an AD or a coach.
pub fn parse_admin_role(label: &str) -> Option<CoachRole> {
    let lowered = label.trim().to_ascii_lowercase();
    if !(lowered.contains("athletic director") || lowered.contains("activities director")) {
        return None;
    }
    if NON_DIRECTOR_TOKENS
        .iter()
        .any(|token| lowered.contains(token))
    {
        return None;
    }
    Some(CoachRole::AthleticDirector)
}

/// Strip leading honorifics so "Coach Smith" and "Smith" mint the same coach identity.
pub fn strip_honorific(value: &str) -> String {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let stripped = parts
        .iter()
        .take_while(|part| {
            matches!(
                part.trim_end_matches('.').to_ascii_lowercase().as_str(),
                "mr" | "mrs" | "ms" | "miss" | "dr" | "coach" | "sir" | "rev"
            )
        })
        .count();
    match parts.get(stripped..) {
        Some(kept) if !kept.is_empty() => kept.join(" "),
        _ => clean(value),
    }
}

// -------------------------------------------------------------------------------------------------
// Entity mapping
// -------------------------------------------------------------------------------------------------

/// Build the canonical school and its AD/head-coach rows for one directory page.
///
/// The school name comes from the page (`JumboMain`), falling back to the index row's `title`
/// attribute when the page omits it. Returns `None` when neither carries a name: nothing is minted
/// from an empty document.
pub fn school_entities(
    entry: &IndexEntry,
    page: &SchoolPage,
    observed_on: &str,
) -> Option<SchoolExtract> {
    let name = meaningful(&page.name).or_else(|| meaningful(&entry.name))?;
    let page_url = entry.page_url();
    let source = SourceRef::new(SOURCE_ID, Some(page_url.clone()));
    let namespace = SourceNamespace::AssociationSchool {
        association: ASSOCIATION.to_string(),
    };

    let (mut school, school_id) = CanonicalSchool::new("WI", &name, normalize_name(&name));
    school.city = page
        .city
        .as_deref()
        .and_then(meaningful)
        .or_else(|| meaningful(&entry.city));
    if let Some(city) = school.city.as_ref() {
        school.aliases.push(format!("{city} WI"));
    }
    school.association = Some(ASSOCIATION.to_string());
    school.classification = page.conference.as_deref().and_then(meaningful);
    school.enrollment = page.enrollment;
    school.school_website = page.website.as_deref().and_then(meaningful);
    school.source_identities.push(
        SourceIdentity::new(namespace.clone(), entry.org_id.clone()).with_url(page_url.clone()),
    );
    school
        .evidence
        .push(Evidence::parsed(source.clone(), observed_on));

    let mut coaches: Vec<CanonicalCoach> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut skipped_admin_roles: Vec<String> = Vec::new();

    for admin in &page.admins {
        let Some(role) = parse_admin_role(&admin.role) else {
            if !skipped_admin_roles.iter().any(|seen| seen == &admin.role) {
                skipped_admin_roles.push(admin.role.clone());
            }
            continue;
        };
        let person = strip_honorific(&admin.name);
        if person.is_empty() {
            continue;
        }
        let mut coach = CanonicalCoach::new(&school_id, person, None, Gender::Mixed, role);
        coach.professional_email = admin.email.as_deref().and_then(valid_email);
        coach
            .evidence
            .push(Evidence::parsed(source.clone(), observed_on));
        if seen.insert(coach.id.to_string()) {
            coaches.push(coach);
        }
    }

    let mut skipped_coach_rows = 0usize;
    for row in &page.coaches {
        let Some((sport, gender)) = parse_sport_label(&row.sport) else {
            skipped_coach_rows = skipped_coach_rows.saturating_add(1);
            continue;
        };
        let Some(role) = parse_coach_role(&row.role) else {
            skipped_coach_rows = skipped_coach_rows.saturating_add(1);
            continue;
        };
        let person = strip_honorific(&row.name);
        if person.is_empty() {
            skipped_coach_rows = skipped_coach_rows.saturating_add(1);
            continue;
        }
        let mut coach = CanonicalCoach::new(&school_id, person, Some(sport), gender, role);
        coach.professional_email = row.email.as_deref().and_then(valid_email);
        coach
            .evidence
            .push(Evidence::parsed(source.clone(), observed_on));
        if seen.insert(coach.id.to_string()) {
            coaches.push(coach);
        }
    }

    Some(SchoolExtract {
        school,
        coaches,
        skipped_admin_roles,
        skipped_coach_rows,
    })
}
