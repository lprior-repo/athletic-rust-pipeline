
use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, ensure, Context, Result};
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam,
};
use census_report::bests::{self, PrKey, SharedSelection};
use census_report::report::{self, Census, Scope};
use census_store::{Store, Table};

use super::fixtures::Corpus;
use super::constants;

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

pub fn assert_scope_split(store: &Store, core: &Census, all_sources: &Census) -> Result<()> {
    ensure!(
        core.scope == "core" && all_sources.scope == "all_sources",
        "the censuses report scopes {:?} and {:?}",
        core.scope,
        all_sources.scope
    );
    let mut athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    let dropped_athletes = report::retain_core(&mut athletes);
    let mut meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    let dropped_meets = report::retain_core(&mut meets);
    ensure!(
        dropped_athletes > 0 && dropped_meets > 0,
        "the store holds no non-core-only row, so the scope split proves nothing"
    );
    ensure!(
        all_sources.totals.athletes == core.totals.athletes + dropped_athletes,
        "the core scope serves {} athletes against {} all-sources and {dropped_athletes} \
         non-core-only athletes",
        core.totals.athletes,
        all_sources.totals.athletes
    );
    ensure!(
        all_sources.meets.total == core.meets.total + dropped_meets,
        "the core scope reports {} meets against {} all-sources and {dropped_meets} non-core-only \
         meets",
        core.meets.total,
        all_sources.meets.total
    );
    ensure!(
        core.totals.athletes > 0 && core.meets.total > 0,
        "the core scope serves no athlete or no meet at all"
    );
    Ok(())
}

pub fn assert_best_reduction(
    rows: &[SharedSelection],
    store: &Store,
    scope: Scope,
) -> Result<()> {
    let mut athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
    let mut meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    let mut events: Vec<CanonicalEvent> = store.scan(Table::Events)?;
    let mut performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    if scope == Scope::Core {
        report::retain_core(&mut athletes);
        report::retain_core(&mut meets);
        report::retain_core(&mut events);
        report::retain_core(&mut performances);
    }

    let cohort: BTreeMap<&str, i16> = athletes
        .iter()
        .map(|athlete| (athlete.id.as_str(), athlete.grad_year.get()))
        .collect();
    let kinds: BTreeMap<&str, &census_domain::model::EventKind> = events
        .iter()
        .map(|event| (event.id.as_str(), &event.kind))
        .collect();

    let parents: BTreeMap<_, _> = meets.iter().map(|meet| (meet.id.as_str(), meet)).collect();
    let mut expected: BTreeMap<PrKey, (i64, usize)> = BTreeMap::new();
    for performance in &performances {
        let Some(kind) = kinds.get(performance.event.as_str()) else {
            continue;
        };
        if cohort.get(performance.athlete.as_str()) != Some(&constants::COHORT)
            || bests::is_relay(kind)
        {
            continue;
        }
        let Some(measure) = bests::Measure::of(&performance.mark) else {
            continue;
        };
        let Some(value) = measure.value(&performance.mark) else {
            continue;
        };
        let Some(key) = PrKey::from_performance(
            performance, kind, parents.get(performance.meet.as_str()).copied(), measure,
        ) else {
            continue;
        };
        let entry = expected.entry(key).or_insert((0, 0));
        let better = entry.0 == 0 || measure.better(value, entry.0);
        if better {
            entry.0 = value;
        }
        entry.1 += 1;
    }
    ensure!(
        !expected.is_empty(),
        "the store holds no class-of-{} mark in scope, so the reduction proves nothing",
        constants::COHORT
    );
    ensure!(
        rows.len() == expected.len(),
        "the reduction published {} rows for the {} pairs the store implies",
        rows.len(),
        expected.len()
    );
    let mut seen: BTreeSet<&PrKey> = BTreeSet::new();
    for row in rows {
        let key = &row.key;
        ensure!(
            seen.insert(key),
            "the reduction published {} twice for one athlete",
            row.event_label()
        );
        let Some((value, count)) = expected.get(key) else {
            bail!(
                "the reduction published {} for {}, which the store does not imply",
                row.event_label(),
                row.athlete_id()
            );
        };
        ensure!(
            row.population.marks == *count,
            "{} rests on {} marks, the store holds {count}",
            row.event_label(),
            row.population.marks
        );
        ensure!(
            row.value == *value,
            "{} of {} is {}, the store's best is {value}",
            row.event_label(),
            row.athlete_id(),
            row.value
        );
    }
    Ok(())
}

fn ids_of<T>(rows: &[T], id: impl Fn(&T) -> &str) -> BTreeSet<String> {
    rows.iter().map(|row| id(row).to_string()).collect()
}

