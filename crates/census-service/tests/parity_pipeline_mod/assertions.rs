use std::collections::BTreeSet;

use anyhow::{ensure, Context, Result};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam, EventKind,
};
use census_report::bests::SharedSelection;
use census_store::{Store, Table};

use super::constants;
use super::fixtures::Corpus;

pub fn assert_counts(counts: &[(String, usize)], expected: &[(&str, usize)]) -> Result<()> {
    for (table, rows) in expected {
        let observed = counts
            .iter()
            .find(|(name, _)| name == table)
            .map(|(_, count)| *count)
            .with_context(|| format!("consolidate reported no count for {table}"))?;
        ensure!(
            observed == *rows,
            "the store holds {observed} rows for {table}; the corpus implies {rows}"
        );
    }
    Ok(())
}

pub fn assert_result_entities(store: &Store, corpus: &Corpus) -> Result<()> {
    let meets: BTreeSet<String> = store
        .scan::<CanonicalMeet>(Table::Meets)?
        .iter()
        .map(|row| row.id.as_str().to_string())
        .collect();
    let events: BTreeSet<String> = store
        .scan::<CanonicalEvent>(Table::Events)?
        .iter()
        .map(|row| row.id.as_str().to_string())
        .collect();
    let teams: BTreeSet<String> = store
        .scan::<CanonicalTeam>(Table::Teams)?
        .iter()
        .map(|row| row.id.as_str().to_string())
        .collect();
    let athletes: BTreeSet<String> = store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .iter()
        .map(|row| row.id.as_str().to_string())
        .collect();
    let performances: BTreeSet<String> = store
        .scan::<CanonicalPerformance>(Table::Performances)?
        .iter()
        .map(|row| row.id.as_str().to_string())
        .collect();

    let mut expected_meets = ids_of(&corpus.meets, |row| row.id.as_str());
    expected_meets.extend(corpus.expected.meets.iter().cloned());
    let mut expected_teams = ids_of(&corpus.teams, |row| row.id.as_str());
    expected_teams.extend(corpus.expected.teams.iter().cloned());
    let mut expected_athletes = ids_of(&corpus.athletes, |row| row.id.as_str());
    expected_athletes.extend(corpus.expected.athletes.iter().cloned());

    for (table, observed, expected) in [
        ("meets", &meets, &expected_meets),
        ("events", &events, &corpus.expected.events),
        ("teams", &teams, &expected_teams),
        ("athletes", &athletes, &expected_athletes),
        ("performances", &performances, &corpus.expected.performances),
    ] {
        ensure!(
            observed == expected,
            "the store holds {} {table} rows against the {} the fixtures imply; first difference: \
             {:?}",
            observed.len(),
            expected.len(),
            observed.symmetric_difference(expected).next()
        );
    }
    Ok(())
}

pub fn assert_source_bests(rows: &[SharedSelection]) -> Result<()> {
    let mut subjects = BTreeSet::new();
    let mut actual = Vec::new();
    for row in rows.iter().filter(|row| {
        row.athlete.name == "Kingston Penn" && row.key.event_kind == EventKind::Track100m
    }) {
        ensure!(
            subjects.insert(row.athlete_id()),
            "distinct source-owned Kingston Penn subjects collapsed into one best"
        );
        ensure!(
            row.athlete.grad_year == constants::COHORT
                && row.athlete.school.as_deref() == Some("Middleton")
                && row.meet.date == "2025-06-06"
                && row.population.marks == 1,
            "Kingston Penn lost source-owned cohort, affiliation, date or mark accounting"
        );
        actual.push((
            row.source.result_url.as_str(),
            row.source.source_key.as_str(),
            row.result.value,
        ));
    }
    actual.sort_unstable();
    let mut expected = Vec::new();
    for file in ["d1boysstateresults-dash.htm", "d1boysstateresults-dash.txt"] {
        let url = format!("https://www.wiaawi.org/Portals/0/PDF/Results/Track/2025/{file}");
        expected.push((
            url.clone(),
            format!("{url}:100 Meter Dash:preliminaries:7"),
            10_860_000_000,
        ));
        expected.push((
            url.clone(),
            format!("{url}:100 Meter Dash:finals:6"),
            10_890_000_000,
        ));
    }
    expected.sort_unstable();
    ensure!(
        actual
            == expected
                .iter()
                .map(|(url, key, value)| (url.as_str(), key.as_str(), *value))
                .collect::<Vec<_>>(),
        "the retained source subjects changed the four captured Kingston Penn marks: {actual:?}"
    );
    Ok(())
}

fn ids_of<T>(rows: &[T], id: impl Fn(&T) -> &str) -> BTreeSet<String> {
    rows.iter().map(|row| id(row).to_string()).collect()
}
