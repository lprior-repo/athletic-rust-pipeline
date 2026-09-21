use super::{AssessmentWire, DetailRow};
use crate::{
    domain::{
        decision::{self, Decision, ReviewChoice},
        evidence::{EvidenceRef, ProfileEvidence, ResultAttribution, Sport},
        facts::Location as FactLocation,
        identity::{AthleteId, EvidenceDigest},
    },
    runtime::{
        acquisition::ProfileAcquisition,
        protocol::{ReviewOutcome, ReviewVerdict},
        row_protocol::{AcceptanceMethod, RowReport},
    },
};
use anyhow::{bail, Result};
use std::collections::{HashMap, HashSet};
use unicode_normalization::{char::is_combining_mark, UnicodeNormalization};

pub(super) fn verify_profiles(profiles: &[ProfileAcquisition]) -> Result<()> {
    profiles.iter().try_for_each(|acquisition| {
        let Some(profile) = acquisition.profile.as_ref() else {
            return Ok(());
        };
        if profile.profile_url.athlete_id() != profile.athlete_id {
            bail!("profile URL athlete ID contradicts profile evidence ID");
        }
        if profile.documents.is_empty() {
            bail!("profile evidence has no retained source documents");
        }
        let response_documents = acquisition
            .responses
            .iter()
            .map(|receipt| receipt.digest.as_str())
            .collect::<HashSet<_>>();
        if profile
            .documents
            .iter()
            .any(|document| !response_documents.contains(document.as_str()))
        {
            bail!("profile document is absent from retained response receipts");
        }
        check_ref(&profile.name.evidence, &profile.documents)?;
        profile.teams.iter().try_for_each(|team| {
            if team.team_id == 0 {
                bail!("profile team has invalid ID");
            }
            check_ref(&team.name.evidence, &profile.documents)?;
            if let Some(location) = team.location.as_ref() {
                check_ref(&location.evidence, &profile.documents)?;
            }
            Ok(())
        })?;
        profile
            .graduation_years
            .iter()
            .try_for_each(|year| check_ref(&year.evidence, &profile.documents))?;
        profile
            .grades
            .iter()
            .try_for_each(|grade| check_ref(&grade.evidence, &profile.documents))?;
        profile.results.iter().try_for_each(|result| {
            if result.result_id == 0 || result.team_id == 0 || result.meet_id == 0 {
                bail!("profile result has invalid source identifiers");
            }
            check_ref(&result.evidence, &profile.documents)
        })?;
        profile.issues.iter().try_for_each(|issue| {
            if let Some(reference) = issue.evidence.as_ref() {
                check_ref(reference, &profile.documents)?;
            }
            if hard_contradiction(&issue.code) {
                bail!(
                    "profile retains hard identity contradiction: {}",
                    issue.code
                );
            }
            Ok(())
        })
    })
}

fn check_ref(reference: &EvidenceRef, documents: &[EvidenceDigest]) -> Result<()> {
    if reference.locator.trim().is_empty() {
        bail!("source evidence locator is empty");
    }
    if !documents
        .iter()
        .any(|document| document == &reference.document)
    {
        bail!("source evidence document is not retained by profile");
    }
    Ok(())
}

fn hard_contradiction(code: &str) -> bool {
    matches!(
        code,
        "identity_conflict"
            | "html_identity_mismatch"
            | "html_identity_inconsistency"
            | "team_conflict"
            | "team_name_inconsistency"
            | "team_location_inconsistency"
            | "identity_incomplete"
            | "profile_url_conflict"
            | "team_history_join_missing"
    )
}

pub(super) fn verify_positive(
    row: &DetailRow,
    report: &RowReport,
    assessment: &AssessmentWire,
    acquisitions: Vec<ProfileAcquisition>,
    selected: AthleteId,
    method: AcceptanceMethod,
) -> Result<()> {
    if !matches!(
        assessment.search,
        crate::domain::decision::SearchCompleteness::Complete { .. }
    ) {
        bail!("accepted result lacks complete search evidence");
    }
    if !assessment
        .candidates
        .iter()
        .any(|candidate| candidate.athlete_id() == selected)
    {
        bail!("accepted athlete was not supplied as an assessment candidate");
    }
    let acquisition = acquisitions
        .iter()
        .find(|item| item.athlete_id == selected)
        .ok_or_else(|| anyhow::anyhow!("accepted athlete lacks retained profile acquisition"))?;
    let profile = acquisition
        .profile
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("accepted athlete lacks retained profile evidence"))?;
    if !acquisition.complete || !acquisition.failures.is_empty() {
        bail!("accepted athlete profile acquisition is incomplete or failed");
    }
    verify_profiles(std::slice::from_ref(acquisition))?;
    verify_identity(row, profile)?;
    verify_participation(profile)?;
    match method {
        AcceptanceMethod::Deterministic => {
            if report.review.is_some() {
                bail!("deterministic acceptance carries a review outcome");
            }
            let corroborated = acquisitions
                .iter()
                .filter(|candidate| candidate_is_corroborated(row, candidate))
                .count();
            if corroborated != 1 {
                bail!("deterministic acceptance lacks a unique corroborated identity");
            }
            verify_deterministic(assessment, selected)?;
        }
        AcceptanceMethod::LocalReview => {
            verify_local_review(report, profile, selected)?;
            verify_local_authorization(row, assessment, acquisitions, selected)?;
        }
    }
    Ok(())
}

fn verify_identity(row: &DetailRow, profile: &ProfileEvidence) -> Result<()> {
    let first = source_field(row, "Person First")?;
    let last = source_field(row, "Person Last")?;
    let school = normalized_school(&source_field(row, "Schools Name")?);
    let city = source_field_optional(row, "Address Mailing / Permanent City");
    let region = source_field_optional(row, "Address Mailing / Permanent Region");
    if normalized(&format!("{first} {last}")).is_empty()
        || matches!(
            school.as_str(),
            "" | "unknown" | "not provided" | "none" | "null" | "n a" | "other" | "high school"
        )
    {
        bail!("accepted source lacks meaningful name or school identity");
    }
    if city.is_none() && region.is_none() {
        bail!("accepted source has no location context");
    }
    if normalized(&format!("{first} {last}")) != normalized(profile.name.value.as_str()) {
        bail!("retained profile name contradicts source identity");
    }
    let city = city.as_deref().map(normalized);
    let region = region.as_deref().map(normalized);
    let mut corroboration = HashMap::new();
    if !profile
        .teams
        .iter()
        .filter(|team| normalized_school(team.name.value.as_str()) == school)
        .any(|team| {
            let Some(observation) = team.location.as_ref() else {
                return false;
            };
            let fields = corroboration
                .entry(team.team_id)
                .or_insert((city.is_none(), region.is_none()));
            let observed = location_matches(&observation.value, city.as_deref(), region.as_deref());
            fields.0 |= observed.0;
            fields.1 |= observed.1;
            fields.0 && fields.1
        })
    {
        bail!("retained profile lacks source-matching school and location evidence");
    }
    Ok(())
}

fn source_field(row: &DetailRow, name: &str) -> Result<String> {
    source_field_optional(row, name)
        .ok_or_else(|| anyhow::anyhow!("accepted source field is empty: {name}"))
}

fn source_field_optional(row: &DetailRow, name: &str) -> Option<String> {
    row.source
        .fields
        .get(name)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn location_matches(
    location: &FactLocation,
    city: Option<&str>,
    region: Option<&str>,
) -> (bool, bool) {
    match location {
        FactLocation::Missing => (false, false),
        FactLocation::RegionOnly(value) => (
            false,
            region.is_some_and(|actual| normalized(value.as_str()) == actual),
        ),
        FactLocation::CityOnly(value) => (
            city.is_some_and(|actual| normalized(value.as_str()) == actual),
            false,
        ),
        FactLocation::CityRegion {
            city: observed_city,
            region: observed_region,
        } => (
            city.is_some_and(|actual| normalized(observed_city.as_str()) == actual),
            region.is_some_and(|actual| normalized(observed_region.as_str()) == actual),
        ),
    }
}

fn normalized(raw: &str) -> String {
    raw.nfkd()
        .filter(|character| !is_combining_mark(*character))
        .fold(String::new(), |mut output, character| {
            if character.is_alphanumeric() {
                output.extend(character.to_lowercase());
            } else if !output.ends_with(' ') {
                output.push(' ');
            }
            output
        })
        .trim()
        .to_owned()
}

fn normalized_school(raw: &str) -> String {
    let mut value = normalized(raw);
    let suffix = " high school";
    if value.ends_with(suffix) {
        value.truncate(value.len().saturating_sub(suffix.len()));
    }
    value
}

fn verify_participation(profile: &ProfileEvidence) -> Result<()> {
    let attributed = profile.results.iter().any(|result| {
        result.result_id > 0
            && matches!(result.sport, Sport::TrackField | Sport::CrossCountry)
            && match &result.attribution {
                ResultAttribution::Individual => true,
                ResultAttribution::VerifiedRelayMember { relay_athlete_id } => {
                    AthleteId::new(*relay_athlete_id).is_ok()
                }
                ResultAttribution::Unresolved { .. } => false,
            }
    });
    if !attributed {
        bail!("accepted athlete lacks attributed TF/XC participation");
    }
    Ok(())
}
fn candidate_is_corroborated(row: &DetailRow, acquisition: &ProfileAcquisition) -> bool {
    let Some(profile) = acquisition.profile.as_ref() else {
        return false;
    };
    acquisition.complete
        && acquisition.failures.is_empty()
        && verify_profiles(std::slice::from_ref(acquisition)).is_ok()
        && verify_identity(row, profile).is_ok()
        && verify_participation(profile).is_ok()
}

fn verify_deterministic(assessment: &AssessmentWire, selected: AthleteId) -> Result<()> {
    if !matches!(
        &assessment.search,
        crate::domain::decision::SearchCompleteness::Complete { .. }
    ) {
        bail!("deterministic acceptance lacks complete search evidence");
    }
    if assessment.decision != crate::domain::decision::Decision::DeterministicAccepted {
        bail!("deterministic acceptance has a non-deterministic assessment");
    }
    if assessment.verified.as_ref().map(|value| value.athlete_id) != Some(selected) {
        bail!("deterministic assessment verified selection does not match report");
    }
    Ok(())
}

fn verify_local_review(
    report: &RowReport,
    profile: &ProfileEvidence,
    selected: AthleteId,
) -> Result<()> {
    let Some(ReviewOutcome::Reviewed {
        verdict:
            ReviewVerdict::Select {
                athlete_id,
                evidence,
                ..
            },
        ..
    }) = report.review.as_ref()
    else {
        bail!("local acceptance missing retained review outcome prerequisite");
    };
    if *athlete_id != selected || evidence.is_empty() {
        bail!("local review selection does not bind selected athlete");
    }
    let allowed = super::profile_refs(profile);
    if evidence
        .iter()
        .any(|reference| !allowed.iter().any(|item| item == reference))
    {
        bail!("local review cites unsupported profile evidence");
    }
    Ok(())
}

fn verify_local_authorization(
    row: &DetailRow,
    assessment: &AssessmentWire,
    acquisitions: Vec<ProfileAcquisition>,
    selected: AthleteId,
) -> Result<()> {
    if assessment.decision != Decision::IdentityReview {
        bail!("local acceptance lacks an identity-review assessment");
    }
    if acquisitions
        .iter()
        .any(|acquisition| !acquisition.complete || !acquisition.failures.is_empty())
    {
        bail!("local acceptance contains incomplete profile discovery");
    }
    let profiles = acquisitions
        .into_iter()
        .map(|acquisition| {
            acquisition
                .profile
                .ok_or_else(|| anyhow::anyhow!("local acceptance lacks a candidate profile"))
        })
        .collect::<Result<Vec<_>>>()?;
    let recomputed = decision::assess(
        &row.source,
        profiles
            .iter()
            .map(crate::domain::candidate::CandidateEvidence::Complete),
        assessment.search.clone(),
    )?;
    if recomputed.decision() != Decision::IdentityReview {
        bail!("retained source evidence does not require identity review");
    }
    let authorized = decision::apply_review(&recomputed, ReviewChoice::Select(selected))?;
    if authorized.accepted_athlete_id() != Some(selected) {
        bail!("retained source evidence does not authorize the local selection");
    }
    Ok(())
}
