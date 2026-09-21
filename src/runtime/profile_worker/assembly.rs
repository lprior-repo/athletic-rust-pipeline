use super::team::TeamObservation;
use crate::domain::evidence::{EvidenceIssue, Observed, ProfileEvidence, TeamEvidence};
use crate::profile;
use anyhow::Result;

pub(crate) struct Assembly {
    pub(crate) profile: Option<ProfileEvidence>,
    pub(crate) complete: bool,
}

pub(crate) fn assemble(
    mut profiles: Vec<ProfileEvidence>,
    html: Option<profile::HtmlProfileEvidence>,
    observations: Vec<TeamObservation>,
    issues: Vec<EvidenceIssue>,
    mut complete: bool,
) -> Result<Assembly> {
    if let Some(html) = &html {
        profiles
            .iter_mut()
            .for_each(|profile| profile.profile_url = html.profile_url.clone());
    }
    let merged = profiles
        .drain(..)
        .try_fold(None, |current, next| match current {
            None => Ok(Some(next)),
            Some(left) => profile::merge_profiles(left, next).map(Some),
        })?;
    let Some(profile) = merged else {
        return Ok(Assembly {
            profile: None,
            complete: false,
        });
    };
    let profile = apply_html(profile, html, issues, &mut complete);
    let profile = observations
        .into_iter()
        .fold(profile, |profile, observation| {
            attach_team(profile, observation, &mut complete)
        });
    Ok(Assembly {
        profile: Some(profile),
        complete,
    })
}

fn apply_html(
    mut profile: ProfileEvidence,
    html: Option<profile::HtmlProfileEvidence>,
    issues: Vec<EvidenceIssue>,
    complete: &mut bool,
) -> ProfileEvidence {
    profile.issues.extend(issues);
    let Some(html) = html else {
        *complete = false;
        return profile;
    };
    if html
        .identity_hints
        .iter()
        .any(|hint| hint.value != profile.name.value)
    {
        *complete = false;
    }
    profile.issues.extend(
        html.identity_hints
            .into_iter()
            .filter(|hint| hint.value != profile.name.value)
            .map(|hint| EvidenceIssue {
                code: "html_identity_inconsistency".to_owned(),
                message: "scoped HTML identity conflicts with bio identity".to_owned(),
                evidence: Some(hint.evidence),
            }),
    );
    html.cohort_witnesses.into_iter().for_each(|witness| {
        if !profile.graduation_years.contains(&witness) {
            profile.graduation_years.push(witness);
        }
    });
    if !profile.documents.contains(&html.document) {
        profile.documents.push(html.document.clone());
    }
    profile.profile_url = html.profile_url;
    if profile
        .issues
        .iter()
        .any(|issue| !issue.is_descriptive_cohort())
    {
        *complete = false;
    }
    profile
}

fn attach_team(
    mut profile: ProfileEvidence,
    observation: TeamObservation,
    complete: &mut bool,
) -> ProfileEvidence {
    retain_team_document(&mut profile, &observation);
    let (has_history, name_conflict, location_conflict) = team_join_state(&profile, &observation);
    if !has_history {
        profile.issues.push(EvidenceIssue {
            code: "team_history_join_missing".to_owned(),
            message: "TeamNav response has no matching bio team history".to_owned(),
            evidence: Some(observation.evidence),
        });
        *complete = false;
        return profile;
    }
    record_team_conflicts(&mut profile, &observation, name_conflict, location_conflict);
    *complete &= !name_conflict && !location_conflict;
    profile.teams.push(TeamEvidence {
        team_id: observation.requested.team_id,
        name: Observed {
            value: observation.name,
            evidence: observation.evidence.clone(),
        },
        location: observation.location.map(|value| Observed {
            value,
            evidence: observation.evidence,
        }),
        seasons: vec![observation.requested.season],
        level: observation.level,
    });
    profile
}

fn retain_team_document(profile: &mut ProfileEvidence, observation: &TeamObservation) {
    if !profile.documents.contains(&observation.evidence.document) {
        profile
            .documents
            .push(observation.evidence.document.clone());
    }
}

/// Whether the observation joins the retained team history, and its two conflict verdicts.
fn team_join_state(profile: &ProfileEvidence, observation: &TeamObservation) -> (bool, bool, bool) {
    let joined = profile
        .teams
        .iter()
        .filter(|team| team.team_id == observation.requested.team_id);
    if joined.clone().next().is_none() {
        return (false, false, false);
    }
    let name_conflict = joined
        .clone()
        .any(|team| team.name.value != observation.name);
    let location_conflict = observation.location.as_ref().is_some_and(|location| {
        joined.clone().any(|team| {
            team.location
                .as_ref()
                .is_some_and(|known| !known.value.compatible_with(location))
        })
    });
    (true, name_conflict, location_conflict)
}

fn record_team_conflicts(
    profile: &mut ProfileEvidence,
    observation: &TeamObservation,
    name_conflict: bool,
    location_conflict: bool,
) {
    [
        (
            name_conflict,
            "team_name_inconsistency",
            "TeamNav name conflicts with retained team name",
        ),
        (
            location_conflict,
            "team_location_inconsistency",
            "TeamNav location conflicts with retained team location",
        ),
    ]
    .into_iter()
    .filter(|(conflict, _, _)| *conflict)
    .for_each(|(_, code, message)| {
        profile.issues.push(EvidenceIssue {
            code: code.to_owned(),
            message: message.to_owned(),
            evidence: Some(observation.evidence.clone()),
        });
    });
}
