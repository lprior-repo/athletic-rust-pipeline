use super::CandidateReason;
use crate::domain::identity::AthleteId;
use crate::domain::{evidence::ProfileEvidence, facts::Location};
use crate::model::SourceRecord;
use std::collections::{BTreeMap, BTreeSet};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

const FIRST_NAME: &str = "Person First";
const LAST_NAME: &str = "Person Last";
const MAILING_CITY: &str = "Address Mailing / Permanent City";
const MAILING_REGION: &str = "Address Mailing / Permanent Region";
const SCHOOL: &str = "Schools Name";

#[derive(Debug, Clone)]
pub(super) struct SourceIdentity {
    pub(super) name: Option<String>,
    pub(super) school: Option<String>,
    mailing_city: Option<String>,
    mailing_region: Option<String>,
}

#[derive(Debug, Clone, Copy)]
struct CandidateFacts {
    exact_name: bool,
    matching_school: bool,
    location_corroborated: bool,
    mailing_location_matches: bool,
    hard_eligible: bool,
    name_conflict: bool,
    participation_confirmed: bool,
    evidence_conflict: bool,
}

pub(super) fn source_identity(record: &SourceRecord) -> SourceIdentity {
    let first = source_value(record, FIRST_NAME);
    let last = source_value(record, LAST_NAME);
    let name = match (first, last) {
        (Some(first), Some(last)) => normalized(&format!("{first} {last}")),
        _ => None,
    };
    SourceIdentity {
        name,
        school: source_value(record, SCHOOL)
            .and_then(|value| normalized(&value))
            .filter(|value| meaningful_school(value)),
        mailing_city: source_value(record, MAILING_CITY).and_then(|value| normalized(&value)),
        mailing_region: source_value(record, MAILING_REGION).and_then(|value| normalized(&value)),
    }
}

fn source_value(record: &SourceRecord, field: &str) -> Option<String> {
    record
        .fields
        .get(field)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn normalized(raw: &str) -> Option<String> {
    let value = raw
        .nfkd()
        .filter(|character| !is_combining_mark(*character))
        .fold(String::new(), |mut output, character| {
            if character.is_alphanumeric() {
                output.extend(character.to_lowercase());
            } else if !output.ends_with(' ') {
                output.push(' ');
            }
            output
        });
    let value = value.trim().to_owned();
    (!value.is_empty()).then_some(value)
}

fn meaningful_school(value: &str) -> bool {
    !matches!(
        value,
        "unknown" | "not provided" | "none" | "null" | "n a" | "other"
    )
}

pub(super) fn group_profiles(
    profiles: &[ProfileEvidence],
) -> BTreeMap<AthleteId, Vec<&ProfileEvidence>> {
    profiles
        .iter()
        .fold(BTreeMap::new(), |mut groups, profile| {
            groups.entry(profile.athlete_id).or_default().push(profile);
            groups
        })
}

pub(super) fn candidate_reason(
    athlete_id: AthleteId,
    profiles: &[&ProfileEvidence],
    source: &SourceIdentity,
) -> CandidateReason {
    let facts = candidate_facts(profiles, source);
    CandidateReason {
        athlete_id,
        exact_name: facts.exact_name,
        matching_school: facts.matching_school,
        location_corroborated: facts.location_corroborated,
        hard_eligible: facts.hard_eligible,
        participation_confirmed: facts.participation_confirmed,
        evidence_conflict: facts.evidence_conflict,
        evidence_strength: strength(facts),
        reasons: candidate_reasons(facts),
        evidence: evidence_digests(profiles),
        mailing_location_matches: facts.mailing_location_matches,
    }
}

fn candidate_facts(profiles: &[&ProfileEvidence], source: &SourceIdentity) -> CandidateFacts {
    let names = profiles
        .iter()
        .filter_map(|profile| normalized(profile.name.value.as_str()))
        .collect::<BTreeSet<_>>();
    let exact_name = source
        .name
        .as_ref()
        .is_some_and(|name| names.contains(name));
    let name_conflict = names.len() > 1;
    let matching_school = source.school.as_ref().is_some_and(|school| {
        profiles.iter().any(|profile| {
            profile
                .teams
                .iter()
                .any(|team| normalized(team.name.value.as_str()).as_ref() == Some(school))
        })
    });
    let mailing_location_matches = school_location_matches(profiles, source);
    let location_corroborated = mailing_location_matches;
    let participation_confirmed = profiles.iter().any(|profile| {
        profile.results.iter().any(|result| {
            result.result_id > 0
                && matches!(
                    result.attribution,
                    crate::domain::evidence::ResultAttribution::Individual
                        | crate::domain::evidence::ResultAttribution::VerifiedRelayMember { .. }
                )
        })
    });
    let evidence_conflict = profiles
        .iter()
        .flat_map(|profile| &profile.issues)
        .any(|issue| {
            matches!(
                issue.code.as_str(),
                "identity_conflict"
                    | "html_identity_mismatch"
                    | "team_conflict"
                    | "identity_incomplete"
                    | "profile_url_conflict"
            )
        });
    let hard_eligible = exact_name
        && !name_conflict
        && matching_school
        && location_corroborated
        && participation_confirmed
        && !evidence_conflict;
    CandidateFacts {
        exact_name,
        matching_school,
        location_corroborated,
        mailing_location_matches,
        hard_eligible,
        name_conflict,
        participation_confirmed,
        evidence_conflict,
    }
}

fn school_location_matches(profiles: &[&ProfileEvidence], source: &SourceIdentity) -> bool {
    source.school.as_ref().is_some_and(|school| {
        profiles.iter().any(|profile| {
            profile.teams.iter().any(|team| {
                if normalized(team.name.value.as_str()).as_deref() != Some(school) {
                    return false;
                }
                team.location.as_ref().is_some_and(|observed| {
                    location_matches(
                        &observed.value,
                        source.mailing_city.as_deref(),
                        source.mailing_region.as_deref(),
                    )
                })
            })
        })
    })
}

fn location_matches(location: &Location, city: Option<&str>, region: Option<&str>) -> bool {
    match location {
        Location::CityOnly(value) => {
            region.is_none() && city == normalized(value.as_str()).as_deref()
        }
        Location::RegionOnly(value) => {
            city.is_none() && region == normalized(value.as_str()).as_deref()
        }
        Location::CityRegion {
            city: school_city,
            region: school_region,
        } => {
            (city.is_some() || region.is_some())
                && city
                    .is_none_or(|value| normalized(school_city.as_str()).as_deref() == Some(value))
                && region.is_none_or(|value| {
                    normalized(school_region.as_str()).as_deref() == Some(value)
                })
        }
        Location::Missing => false,
    }
}

fn strength(facts: CandidateFacts) -> u8 {
    match [
        facts.exact_name,
        facts.matching_school,
        facts.location_corroborated,
        facts.participation_confirmed,
    ]
    .into_iter()
    .filter(|value| *value)
    .count()
    {
        0 => 0,
        1 => 25,
        2 => 50,
        3 => 75,
        _ => 100,
    }
}

fn candidate_reasons(facts: CandidateFacts) -> Vec<String> {
    [
        (facts.exact_name, "exact normalized full name"),
        (facts.matching_school, "meaningful exact normalized school match"),
        (facts.location_corroborated, "matching school has observed location corroboration"),
        (facts.mailing_location_matches, "mailing location is compatible corroboration only"),
        (
            facts.matching_school && !facts.mailing_location_matches,
            "mailing location is not athlete residence and does not disprove school geography",
        ),
        (true, "eligibility established by source workbook membership; no grade or graduation-year requirement"),
        (facts.name_conflict, "conflicting observed profile names"),
        (facts.participation_confirmed, "independently attributed TF or XC participation"),
        (!facts.participation_confirmed, "missing independently attributed sport participation"),
        (facts.evidence_conflict, "source identity evidence is contradictory"),
    ]
    .into_iter()
    .filter(|(present, _)| *present)
    .map(|(_, reason)| reason.to_owned())
    .collect()
}

fn evidence_digests(profiles: &[&ProfileEvidence]) -> Vec<crate::domain::identity::EvidenceDigest> {
    profiles
        .iter()
        .flat_map(|profile| {
            let team_refs = profile.teams.iter().flat_map(|team| {
                team.location
                    .iter()
                    .map(|location| &location.evidence.document)
                    .chain(std::iter::once(&team.name.evidence.document))
            });
            let result_refs = profile
                .results
                .iter()
                .map(|result| &result.evidence.document);
            let grade_refs = profile.grades.iter().map(|grade| &grade.evidence.document);
            let year_refs = profile
                .graduation_years
                .iter()
                .map(|year| &year.evidence.document);
            profile
                .documents
                .iter()
                .chain(team_refs)
                .chain(result_refs)
                .chain(grade_refs)
                .chain(year_refs)
        })
        .fold(BTreeMap::new(), |mut unique, digest| {
            unique
                .entry(digest.as_str().to_owned())
                .or_insert_with(|| digest.clone());
            unique
        })
        .into_values()
        .collect()
}
