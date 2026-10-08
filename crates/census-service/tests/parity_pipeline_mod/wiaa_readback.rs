use std::collections::BTreeMap;

use anyhow::{ensure, Context, Result};
use census_crawl::result_file::{ParsedEvent, ParsedMeet, ParsedRow};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam, Evidence,
    EvidenceMethod, GradYear, Grade, SchoolYear, SourceNamespace, SourceObservation, Sport,
};
use census_store::{Store, Table};

use super::fixtures::Corpus;

pub fn assert_source_results(store: &Store, corpus: &Corpus) -> Result<()> {
    let athletes: BTreeMap<_, _> = store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .into_iter()
        .map(|row| (row.id.clone(), row))
        .collect();
    let teams: BTreeMap<_, _> = store
        .scan::<CanonicalTeam>(Table::Teams)?
        .into_iter()
        .map(|row| (row.id.clone(), row))
        .collect();
    let performances: BTreeMap<_, _> = store
        .scan::<CanonicalPerformance>(Table::Performances)?
        .into_iter()
        .map(|row| (row.source_key.clone(), row))
        .collect();
    let meets = store.scan::<CanonicalMeet>(Table::Meets)?;
    let events = store.scan::<CanonicalEvent>(Table::Events)?;
    for (url, published, sport) in &corpus.published_results {
        let meet = meets
            .iter()
            .find(|meet| meet.source_urls.contains(url))
            .with_context(|| format!("retained meet missing for {url}"))?;
        ensure!(
            meet.name == published.name
                && meet.date == published.date
                && meet.end_date == published.end_date
                && meet.sports.contains(sport),
            "{url}: published meet context changed"
        );
        assert_capture_clock(&meet.evidence, url)?;
        if published.date.len() == 4 {
            assert_undated_participants(store, published, url, *sport)?;
            ensure!(
                !events.iter().any(|event| event.meet == meet.id)
                    && !performances.values().any(|row| row.meet == meet.id),
                "{url}: a year-only source must not manufacture dated event performances"
            );
            continue;
        }
        let school_year = source_school_year(&published.date, *sport)?;
        for event in &published.events {
            let stored = events
                .iter()
                .find(|stored| {
                    stored.meet == meet.id
                        && stored.kind == event.kind
                        && stored.gender == event.gender
                        && stored.division == event.division
                        && stored.round == event.round
                })
                .with_context(|| format!("{url}: published event {:?} missing", event.label))?;
            assert_event(stored, event, url)?;
            for (ordinal, row) in event.rows.iter().enumerate() {
                for (position, name, grade) in members(row) {
                    let Some(grade) = grade else { continue };
                    if row.school.trim().is_empty() || name.trim().is_empty() {
                        continue;
                    }
                    let round = event.round.as_deref().map_or("<none>", |round| round);
                    let mut key = format!("{url}:{}:{round}:{ordinal}", event.label);
                    if let Some(position) = position {
                        key.push_str(&format!(":leg{position}"));
                    }
                    let performance = performances
                        .get(&key)
                        .with_context(|| format!("published participant missing: {key}"))?;
                    assert_performance(performance, row, grade, url)?;
                    ensure!(
                        performance.event == stored.id
                            && performance.meet == meet.id
                            && performance.date == published.date
                            && performance.round == event.round,
                        "{key}: retained event/date/round ownership changed"
                    );
                    let athlete = athletes
                        .get(&performance.athlete)
                        .context("result subject")?;
                    let team = teams
                        .get(&performance.team)
                        .context("historical result team")?;
                    ensure!(
                        athlete.canonical_name == name
                            && athlete.gender == event.gender
                            && Some(athlete.grad_year) == GradYear::of(grade, school_year)
                            && athlete.school == team.school
                            && team.school_year == school_year
                            && team.sport == *sport
                            && team.gender == event.gender,
                        "{key}: source participant, cohort or historical affiliation changed"
                    );
                    ensure!(
                        athlete.observed_grades.iter().any(|observed| {
                            observed.grade == grade
                                && observed.school_year == school_year
                                && observed.source.id == "wiaa_results"
                                && observed.source.url.as_deref() == Some(url)
                        }),
                        "{key}: published dated grade support missing"
                    );
                    assert_capture_clock(&athlete.evidence, url)?;
                }
            }
        }
    }
    Ok(())
}

fn assert_event(stored: &CanonicalEvent, event: &ParsedEvent, url: &str) -> Result<()> {
    ensure!(
        stored.source_labels.iter().any(|label| {
            label.label == event.label
                && label.source.id == "wiaa_results"
                && label.source.url.as_deref() == Some(url)
        }),
        "{url}: exact source-owned event label missing"
    );
    let specification = stored.resolved_specification()?;
    ensure!(
        specification.implement.is_none()
            && specification.hurdles.is_none()
            && specification.indoor_track.is_none()
            && specification.cross_country.is_none(),
        "{url}: the retained headers publish no implement, hurdle, indoor track or course specification"
    );
    assert_capture_clock(&stored.evidence, url)
}

fn assert_performance(
    performance: &CanonicalPerformance,
    row: &ParsedRow,
    grade: Grade,
    url: &str,
) -> Result<()> {
    ensure!(
        performance.mark == row.mark
            && performance.place == row.place
            && performance.wind_mps == row.wind_mps
            && performance.heat == row.heat
            && performance.observed_grade == Some(grade),
        "{}: published mark/status/conditions changed",
        performance.source_key
    );
    let owner = performance
        .source_athlete
        .as_ref()
        .context("source result owner")?;
    ensure!(
        matches!(&owner.namespace, SourceNamespace::Other(namespace) if namespace == "wiaa_result_row")
            && owner.id == performance.source_key,
        "{}: result owner changed",
        performance.source_key
    );
    assert_capture_clock(&performance.evidence, url)
}

fn assert_capture_clock(evidence: &[Evidence], url: &str) -> Result<()> {
    ensure!(
        evidence.iter().any(|item| {
            item.source.id == "wiaa_results"
                && item.source.url.as_deref() == Some(url)
                && item.method == EvidenceMethod::Parsed
                && item.observed_on == super::constants::WIAA_CAPTURED_AT
        }),
        "{url}: parsed evidence must retain the capture clock, not the evaluation clock"
    );
    Ok(())
}

pub(super) fn members(row: &ParsedRow) -> impl Iterator<Item = (Option<u8>, &str, Option<Grade>)> {
    row.legs
        .is_empty()
        .then_some((None, row.name.as_str(), row.grade))
        .into_iter()
        .chain(
            row.legs
                .iter()
                .map(|leg| (Some(leg.position), leg.name.as_str(), leg.grade)),
        )
}

fn source_school_year(date: &str, sport: Sport) -> Result<SchoolYear> {
    if let Some(year) = SchoolYear::from_date(date) {
        return Ok(year);
    }
    ensure!(
        sport == Sport::CrossCountry,
        "year-only non-XC source period: {date}"
    );
    SchoolYear::new(date.parse()?).context("published XC archive school year")
}

fn assert_undated_participants(
    store: &Store,
    published: &ParsedMeet,
    url: &str,
    sport: Sport,
) -> Result<()> {
    let observations = store.scan::<SourceObservation>(Table::SourceObservations)?;
    let school_year = source_school_year(&published.date, sport)?;
    for event in &published.events {
        for (ordinal, row) in event.rows.iter().enumerate() {
            for (position, name, grade) in members(row) {
                if name.trim().is_empty() {
                    continue;
                }
                let round = event.round.as_deref().map_or("<none>", |round| round);
                let mut key = format!("{url}:{}:{round}:{ordinal}", event.label);
                if let Some(position) = position {
                    key.push_str(&format!(":leg{position}"));
                }
                let observation = observations.iter().find_map(|observation| match observation {
                    SourceObservation::Athlete(row)
                        if matches!(&row.namespace, SourceNamespace::Other(namespace) if namespace == "wiaa_result_row")
                            && row.source_athlete_id == key => Some(row),
                    _ => None,
                }).with_context(|| format!("undated native participant lost: {key}"))?;
                ensure!(
                    observation.source_row_key == key
                        && observation.observed_name == name
                        && observation.observed_school.as_deref() == Some(row.school.as_str())
                        && observation.gender == event.gender
                        && observation.observed_on == super::constants::WIAA_CAPTURED_AT
                        && observation
                            .observed_grade
                            .as_ref()
                            .map(|grade| (grade.grade, grade.school_year))
                            == grade.map(|grade| (grade, school_year)),
                    "{key}: undated source facts or capture clock changed"
                );
            }
        }
    }
    Ok(())
}
