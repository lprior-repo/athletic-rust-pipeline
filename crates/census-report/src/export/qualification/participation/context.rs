use crate::export::ExportDataset;
use census_crawl::milesplit::{OwnedPerformance, VerifiedRetainedResultMetadata};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
    CanonicalTeam, EventIdentity, SourceNamespace,
};
use serde_json::Value;

pub(super) fn matches(
    dataset: &ExportDataset,
    performance: &CanonicalPerformance,
    source: &VerifiedRetainedResultMetadata,
    notes: &super::note::SourceNotes,
) -> bool {
    let row = source.result().row();
    let Some(meet) = dataset
        .meets
        .iter()
        .find(|meet| meet.id == performance.meet)
    else {
        return false;
    };
    let Some(event) = dataset
        .events
        .iter()
        .find(|event| event.id == performance.event)
    else {
        return false;
    };
    let Some(team) = dataset.teams.get(&performance.team) else {
        return false;
    };
    let Some(school) = dataset.schools.get(&team.school) else {
        return false;
    };
    performance.id
        == CanonicalPerformance::mint(
            &performance.athlete,
            &performance.meet,
            &performance.event,
            &performance.date,
            &performance.source_key,
        )
        && athlete_matches(dataset, performance, school, row)
        && meet_matches(meet, performance, source, notes)
        && event_matches(event, performance, row)
        && team_matches(dataset, team, school, source)
}

fn meet_matches(
    meet: &CanonicalMeet,
    performance: &CanonicalPerformance,
    source: &VerifiedRetainedResultMetadata,
    notes: &super::note::SourceNotes,
) -> bool {
    let expected_meet = CanonicalMeet::mint(
        Some(notes.reference.site.jurisdiction()),
        &performance.date,
        &source.page().meet.name,
        None,
    );
    meet.id == expected_meet
        && meet.date == performance.date
        && meet.source_identities.iter().any(|identity| {
            identity.namespace == SourceNamespace::MilesplitMeet
                && identity.id == source.result().row().meet_id.to_string()
        })
}

fn event_matches(
    event: &CanonicalEvent,
    performance: &CanonicalPerformance,
    row: &OwnedPerformance,
) -> bool {
    let identity = EventIdentity {
        meet: &performance.meet,
        kind: row.event_kind.clone(),
        gender: row.gender,
        division: text(&row.provider, "divisionName"),
        round: text(&row.provider, "roundName"),
    };
    event.meet == performance.meet
        && event.kind == row.event_kind
        && event.gender == row.gender
        && event.division.as_deref() == identity.division
        && event.round.as_deref() == identity.round
        && CanonicalEvent::new(identity, event.specification.clone())
            .is_ok_and(|expected| expected.id == event.id)
}

fn team_matches(
    dataset: &ExportDataset,
    team: &CanonicalTeam,
    school: &CanonicalSchool,
    source: &VerifiedRetainedResultMetadata,
) -> bool {
    let row = source.result().row();
    let owner = row.team_id.to_string();
    team.gender == row.gender
        && source.page().sport == Some(team.sport)
        && team.school_year == source.page().school_year
        && team.id == CanonicalTeam::mint(&team.school, team.sport, team.gender, team.school_year)
        && team.source_identities.iter().any(|identity| {
            identity.namespace == SourceNamespace::MilesplitTeam && identity.id == owner
        })
        && school.source_identities.iter().any(|identity| {
            identity.namespace == SourceNamespace::MilesplitSchool && identity.id == owner
        })
        && dataset
            .schools
            .values()
            .filter(|school| {
                school.source_identities.iter().any(|identity| {
                    identity.namespace == SourceNamespace::MilesplitSchool && identity.id == owner
                })
            })
            .count()
            == 1
}

fn text<'a>(provider: &'a Value, field: &str) -> Option<&'a str> {
    provider
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
}

fn athlete_matches(
    dataset: &ExportDataset,
    performance: &CanonicalPerformance,
    school: &CanonicalSchool,
    row: &OwnedPerformance,
) -> bool {
    let Some(year) = row.grad_year else {
        return false;
    };
    let name = format!("{} {}", row.first_name.trim(), row.last_name.trim());
    let name = name.trim();
    performance.athlete
        == CanonicalAthlete::mint(&school.id, name, year, row.gender, &row.source_athlete)
        && dataset.athletes.iter().any(|athlete| {
            athlete.id == performance.athlete
                && athlete.school == school.id
                && athlete.canonical_name == name
                && athlete.grad_year == year
                && athlete.gender == row.gender
                && athlete.source.as_ref().is_some_and(|source| {
                    source.namespace == row.source_athlete.namespace
                        && source.id == row.source_athlete.id
                })
        })
}
