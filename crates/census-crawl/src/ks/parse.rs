//! One KSHSAA record as a canonical school and its athletic-director coach.
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SchoolId,
    SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;

use super::wire::KshsaaRecord;

/// Convert one KSHSAA record into a canonical school, if the name is non-empty.
pub fn parse_school(
    record: &KshsaaRecord,
    source_url: &str,
    observed_on: &str,
) -> Option<(CanonicalSchool, SchoolId)> {
    let name = record.school_name.trim();
    if name.is_empty() {
        return None;
    }
    let normalized = normalize_name(name);
    let (mut school, id) = CanonicalSchool::new(UsJurisdiction::Kansas, name, &normalized);
    school.city = nonempty(&record.mailing_city);
    school.association = Some(super::ASSOCIATION.to_string());
    school.classification = record.class.as_ref().and_then(|v| nonempty(v));
    school.enrollment = record.enrollment;
    school.school_website = record.web_site.as_ref().and_then(|v| nonempty(v));
    school.source_identities.push(
        SourceIdentity::new(
            SourceNamespace::AssociationSchool {
                association: super::ASSOCIATION.to_string(),
            },
            &record.identifier,
        )
        .with_url(source_url),
    );
    school.evidence.push(Evidence::parsed(
        SourceRef::new("ks", Some(source_url.to_string())),
        observed_on,
    ));
    Some((school, id))
}

/// Convert one KSHSAA record into an athletic-director coach entity.
///
/// Returns `None` when the AD name is empty or missing.
pub fn parse_ad_coach(
    record: &KshsaaRecord,
    school_id: &SchoolId,
    source_url: &str,
    observed_on: &str,
) -> Option<CanonicalCoach> {
    let name = record.ad_name.as_deref().unwrap_or("").trim();
    if name.is_empty() {
        return None;
    }
    let name = strip_honorific(name);
    let mut coach = CanonicalCoach::new(
        school_id,
        &name,
        None,          // school-wide role
        Gender::Mixed, // AD is not gender-specific
        CoachRole::AthleticDirector,
    );
    if let Some(email) = &record.ad_email {
        coach.set_published_email(email);
    }
    coach.evidence.push(Evidence::parsed(
        SourceRef::new("ks", Some(source_url.to_string())),
        observed_on,
    ));
    Some(coach)
}

/// Strip leading honorifics ("Mr.", "Dr.", "Coach", etc.) from a person name.
fn strip_honorific(value: &str) -> String {
    let mut parts: Vec<&str> = value.split_whitespace().collect();
    while let Some(first) = parts.first() {
        let token = first.trim_end_matches('.').to_ascii_lowercase();
        if matches!(
            token.as_str(),
            "mr" | "mrs" | "ms" | "miss" | "dr" | "coach" | "coach." | "sir" | "rev"
        ) {
            parts.remove(0);
        } else {
            break;
        }
    }
    if parts.is_empty() {
        value.trim().to_string()
    } else {
        parts.join(" ")
    }
}

/// Convert a non-empty trimmed string into `Some`, or `None`.
fn nonempty(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}
