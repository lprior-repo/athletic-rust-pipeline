use super::parse_captured_roster;
use crate::milesplit::{
    parse_team_index, roster_entities, LocatedRosterAthlete, RosterRejectionKind, RosterVerdict,
    Site, TeamRef,
};
use crate::net::cache::content_digest;
use crate::net::FetchOutcome;
use census_domain::model::{CanonicalAthlete, GradYear, SchoolYear, SourceNamespace};
use census_domain::UsJurisdiction;
use serde::Deserialize;

const DIRECTORY: &[u8] =
    include_bytes!("../../../participation_evidence/provider_school/fixtures/mi-team-index.html");
const ROSTER: &[u8] = include_bytes!(
    "../../../participation_evidence/provider_school/fixtures/dcc-provider-roster.html"
);
const MANIFEST: &[u8] = include_bytes!(
    "../../../participation_evidence/provider_school/fixtures/capture-manifest.json"
);
const ROSTER_DIGEST: &str = "246374a87ba6d56802e58e54e4a1edcc5d1eee52106dedb24df185cbd037e178";
const DIRECTORY_DIGEST: &str = "39606bf8e07f0170138bb586ae8a762903bb820b515520e89e34745f6c732351";
const URL: &str = "https://mi.milesplit.com/teams/12439-detroit-catholic-central/roster";
const ACQUIRED: &str = "2026-10-16T00:49:43Z";
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Deserialize)]
struct Manifest {
    version: u32,
    captures: Vec<Capture>,
}

#[derive(Deserialize)]
struct Capture {
    label: String,
    requested_url: String,
    final_url: String,
    status: u16,
    request_method: String,
    started_at: String,
    completed_at: String,
    content_type: String,
    files: Vec<File>,
}

#[derive(Deserialize)]
struct File {
    path: String,
    bytes: usize,
    sha256: String,
}

fn capture() -> TestResult<FetchOutcome> {
    check!(MANIFEST.len() < 16 * 1024);
    check!(eq; ROSTER.len(), 372_977);
    check!(ROSTER.len() < 512 * 1024);
    check!(eq; content_digest(ROSTER), ROSTER_DIGEST);
    let manifest: Manifest = serde_json::from_slice(MANIFEST)?;
    check!(eq; manifest.version, 1);
    let mut captures = manifest
        .captures
        .iter()
        .filter(|item| item.label == "dcc-provider-roster");
    let source = captures.next().ok_or("retained roster capture")?;
    check!(captures.next().is_none());
    check!(eq; source.requested_url, URL);
    check!(eq; source.final_url, URL);
    check!(eq; source.status, 200);
    check!(eq; source.request_method, "GET");
    check!(eq; source.started_at, "2026-10-16T00:49:38Z");
    check!(eq; source.completed_at, ACQUIRED);
    check!(eq; source.content_type, "text/html; charset=UTF-8");
    let mut files = source
        .files
        .iter()
        .filter(|file| file.path == "dcc-provider-roster.html");
    let file = files.next().ok_or("retained roster body")?;
    check!(files.next().is_none());
    check!(eq; file.bytes, ROSTER.len());
    check!(eq; file.sha256, ROSTER_DIGEST);
    Ok(FetchOutcome {
        url: source.requested_url.clone(),
        response_url: Some(source.final_url.clone()),
        method: source.request_method.clone(),
        status: source.status,
        content_digest: file.sha256.clone(),
        bytes: file.bytes,
        fetched_at: source.completed_at.clone(),
        from_cache: false,
        content_type: Some(source.content_type.clone()),
        body: ROSTER.to_vec(),
    })
}

fn team() -> TestResult<TeamRef> {
    check!(eq; DIRECTORY.len(), 293_847);
    check!(eq; content_digest(DIRECTORY), DIRECTORY_DIGEST);
    let directory = parse_team_index(std::str::from_utf8(DIRECTORY)?)?;
    check!(eq; directory.errors, 0);
    check!(directory.unfinished.is_empty());
    let mut teams = directory
        .teams
        .into_iter()
        .filter(|team| team.id == "12439");
    let team = teams.next().ok_or("retained DCC directory owner")?;
    check!(teams.next().is_none());
    check!(eq; format!("{}/roster", team.url), URL);
    Ok(team)
}

#[test]
fn genuine_retained_co2027_rows_project_source_population_without_identity_acceptance() -> TestResult
{
    let capture = capture()?;
    let team = team()?;
    let body = std::str::from_utf8(ROSTER)?;
    let verdict = parse_captured_roster(body, team.clone(), &capture)?;
    let RosterVerdict::Partial {
        roster,
        rejected,
        unfinished,
    } = verdict
    else {
        return Err("genuine roster must retain invalid graduation rows".into());
    };
    check!(unfinished.is_none());
    check!(eq; roster.athletes.len(), 146);
    check!(eq; rejected.len(), 18);
    check!(rejected
        .iter()
        .all(|row| row.kind == RosterRejectionKind::InvalidGraduationYear));
    let mut selected = Vec::new();
    selected.try_reserve_exact(45)?;
    for member in &roster.athletes {
        if member.athlete().grad_year == GradYear::CO2027 {
            check!(selected.len() < 45);
            selected.push(member.clone());
        }
    }
    check!(eq; selected.len(), 45);
    let year = SchoolYear::from_date("2026-10-16").ok_or("capture school year")?;
    let site = Site::for_jurisdiction(UsJurisdiction::Michigan);
    let (_, projected, _) = roster_entities(&team, &selected, year, ACQUIRED, &site)?;
    check!(eq; projected.len(), selected.len());
    for (member, athlete) in selected.iter().zip(&projected) {
        verify_projection(body, member, athlete)?;
    }
    check!(selected
        .windows(2)
        .all(|pair| pair[0].locator().ordinal < pair[1].locator().ordinal));
    check!(eq; content_digest(ROSTER), ROSTER_DIGEST);
    Ok(())
}

fn verify_projection(
    body: &str,
    member: &LocatedRosterAthlete,
    athlete: &CanonicalAthlete,
) -> TestResult {
    let location = member.locator();
    let end = location
        .byte_offset
        .checked_add(location.byte_length)
        .ok_or("row span overflow")?;
    let span = body
        .get(location.byte_offset..end)
        .ok_or("owned UTF-8 roster span")?;
    check!(span.contains(&member.athlete().profile_url));
    check!(span.contains("2027"));
    check!(eq; athlete.grad_year, GradYear::CO2027);
    check!(eq; athlete.canonical_name, member.athlete().name);
    let identity = athlete
        .identity_in(&SourceNamespace::MilesplitAthlete)
        .ok_or("projected provider identity")?;
    check!(eq; identity.id, member.athlete().athlete_id);
    check!(eq; identity.url.as_deref(), Some(member.athlete().profile_url.as_str()));
    let [graduation] = athlete.published_graduations.as_slice() else {
        return Err("one source-published graduation required".into());
    };
    check!(eq; graduation.grad_year, GradYear::CO2027);
    check!(eq; graduation.source.url.as_deref(), Some(URL));
    check!(athlete.observed_grades.is_empty());
    check!(athlete.identity_attestations.is_empty());
    check!(athlete
        .evidence
        .iter()
        .all(|evidence| evidence.observed_on == ACQUIRED));
    check!(eq; athlete.evidence.len(), 1);
    Ok(())
}

#[test]
fn genuine_retained_roster_refuses_a_different_observed_final_owner() -> TestResult {
    let mut capture = capture()?;
    capture.response_url = Some("https://mi.milesplit.com/teams/1-other/roster".into());
    check!(parse_captured_roster(std::str::from_utf8(ROSTER)?, team()?, &capture).is_err());
    Ok(())
}
