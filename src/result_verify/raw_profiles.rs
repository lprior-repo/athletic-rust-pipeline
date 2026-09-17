use super::source_receipts;
use crate::{
    domain::{
        evidence::{EvidenceIssue, ProfileEvidence, Sport},
        identity::AthleteId,
    },
    profile,
    runtime::{
        acquisition::{ProfileAcquisition, ProfileProbe, TeamRequest},
        profile_worker::{assembly, team},
    },
    store::ArtifactStore,
};
use anyhow::{bail, Context, Result};
use url::Url;

mod parsing;
use parsing::{parse_observation, ParsedOperation};

pub(super) fn verify(
    acquisition: &ProfileAcquisition,
    probe: &ProfileProbe,
    origin: &Url,
    store: &ArtifactStore,
) -> Result<()> {
    if !acquisition.complete
        || !acquisition.failures.is_empty()
        || acquisition.athlete_id != probe.athlete_id
        || !acquisition.operations.starts_with(&probe.operations)
        || !acquisition.responses.starts_with(&probe.responses)
        || probe.operations.len() != 3
    {
        bail!("raw profile verification requires a complete acquisition bound to its probe");
    }
    let observations = source_receipts::successful_operations(
        &acquisition.operations,
        &acquisition.responses,
        origin,
        store,
    )?;
    let parsed = observations
        .iter()
        .map(|observation| parse_observation(observation, acquisition.athlete_id, origin, store))
        .collect::<Result<Vec<_>>>()?;
    verify_full_operations(&parsed, acquisition.athlete_id)?;
    verify_parsed_probe(
        probe,
        parsed
            .get(..3)
            .context("full profile lacks its identity probe")?,
    )?;
    let parts = full_parts(parsed);
    let (expected_requests, exceeded) = team::unique_requests(parts.requests);
    if exceeded {
        bail!("raw profile authorization exceeds the bounded TeamNav request set");
    }
    verify_team_order(&parts.teams, &expected_requests)?;
    let assembled =
        assembly::assemble(parts.profiles, parts.html, parts.teams, parts.issues, true)?;
    if !assembled.complete {
        bail!("complete acquisition is contradicted by its raw authorization or evidence");
    }
    let expected = assembled
        .profile
        .context("raw profile receipts contain no assembled profile")?;
    let actual = acquisition
        .profile
        .as_ref()
        .context("raw profile verification has no profile evidence")?;
    if actual != &expected {
        bail!("retained profile differs from exact raw source reconstruction");
    }
    Ok(())
}

pub(super) fn verify_probe(
    probe: &ProfileProbe,
    origin: &Url,
    store: &ArtifactStore,
) -> Result<()> {
    let observations =
        source_receipts::successful_operations(&probe.operations, &probe.responses, origin, store)?;
    let parsed = observations
        .iter()
        .map(|observation| parse_observation(observation, probe.athlete_id, origin, store))
        .collect::<Result<Vec<_>>>()?;
    verify_parsed_probe(probe, &parsed)
}

fn verify_parsed_probe(probe: &ProfileProbe, parsed: &[ParsedOperation]) -> Result<()> {
    verify_initial_subsequence(parsed, probe.athlete_id)?;
    if parsed
        .iter()
        .any(|item| matches!(item, ParsedOperation::Team { .. }))
    {
        bail!("identity probe contains TeamNav operations");
    }
    let profiles = parsed.iter().filter_map(|item| match item {
        ParsedOperation::Bio { profile, .. } => Some(profile.as_ref()),
        _ => None,
    });
    let html = parsed.iter().filter_map(|item| match item {
        ParsedOperation::Html { profile, .. } => Some(profile),
        _ => None,
    });
    let identities = parsed.iter().filter_map(|item| match item {
        ParsedOperation::Bio { identity, .. } => identity.as_ref(),
        _ => None,
    });
    let requests = parsed.iter().flat_map(|item| match item {
        ParsedOperation::Bio { requests, .. } => requests.as_slice(),
        _ => &[],
    });
    let issues = parsed.iter().flat_map(|item| match item {
        ParsedOperation::Bio { issues, .. } => issues.as_slice(),
        ParsedOperation::Html { profile, .. } => profile.issues.as_slice(),
        ParsedOperation::Team { .. } => &[],
    });
    if !profiles.eq(probe.profiles.iter())
        || !html.eq(probe.html.iter())
        || !identities.eq(probe.identities.iter())
        || !requests.eq(probe.requests.iter())
        || !issues.eq(probe.issues.iter())
    {
        bail!("retained identity probe differs from exact raw source reconstruction");
    }
    Ok(())
}

fn verify_initial_subsequence(parsed: &[ParsedOperation], athlete_id: AthleteId) -> Result<()> {
    let expected_html = expected_html_url(athlete_id);
    parsed
        .iter()
        .try_fold(None, |previous, item| {
            let current = match item {
                ParsedOperation::Bio { sport, .. } => match sport {
                    Sport::TrackField => 0,
                    Sport::CrossCountry => 1,
                },
                ParsedOperation::Html { profile_url, .. } => {
                    if profile_url.as_str() != expected_html.as_str() {
                        bail!("HTML receipt is not the production profile route");
                    }
                    2
                }
                ParsedOperation::Team { .. } => 3,
            };
            if previous.is_some_and(|value| current <= value) {
                bail!("identity probe source routes are duplicated or out of order");
            }
            Ok(Some(current))
        })
        .map(|_| ())
}

fn verify_full_operations(parsed: &[ParsedOperation], athlete_id: AthleteId) -> Result<()> {
    let initial = parsed
        .get(..3)
        .context("full profile lacks initial source routes")?;
    let [track_field, cross_country, html] = initial else {
        bail!("full profile lacks initial source routes");
    };
    let expected_html = expected_html_url(athlete_id);
    if !matches!(
        track_field,
        ParsedOperation::Bio {
            sport: Sport::TrackField,
            ..
        }
    ) || !matches!(
        cross_country,
        ParsedOperation::Bio {
            sport: Sport::CrossCountry,
            ..
        }
    ) || !matches!(
        html,
        ParsedOperation::Html { profile_url, .. }
            if profile_url.as_str() == expected_html.as_str()
    ) {
        bail!("full profile source routes are not the production initial TF/XC/HTML order");
    }
    if parsed
        .iter()
        .skip(3)
        .any(|item| !matches!(item, ParsedOperation::Team { .. }))
    {
        bail!("full profile contains a non-TeamNav operation after initial routes");
    }
    Ok(())
}

fn expected_html_url(athlete_id: AthleteId) -> String {
    format!(
        "https://athletic.net/athlete/{}/track-and-field/all",
        athlete_id.get()
    )
}

fn verify_team_order(teams: &[TeamObservation], expected: &[TeamRequest]) -> Result<()> {
    if !teams.iter().map(|team| &team.requested).eq(expected.iter()) {
        bail!("TeamNav operations differ from reparsed authorized request order");
    }
    Ok(())
}

#[derive(Default)]
struct FullParts {
    profiles: Vec<ProfileEvidence>,
    html: Option<profile::HtmlProfileEvidence>,
    teams: Vec<TeamObservation>,
    issues: Vec<EvidenceIssue>,
    requests: Vec<TeamRequest>,
}

fn full_parts(parsed: Vec<ParsedOperation>) -> FullParts {
    parsed
        .into_iter()
        .fold(FullParts::default(), |mut parts, item| {
            match item {
                ParsedOperation::Bio {
                    profile,
                    requests,
                    issues,
                    ..
                } => {
                    parts.profiles.push(*profile);
                    parts.requests.extend(requests);
                    parts.issues.extend(issues);
                }
                ParsedOperation::Html { profile, .. } => {
                    parts.issues.extend(profile.issues.iter().cloned());
                    parts.html = Some(profile);
                }
                ParsedOperation::Team { observation } => parts.teams.push(observation),
            }
            parts
        })
}

type TeamObservation = team::TeamObservation;
