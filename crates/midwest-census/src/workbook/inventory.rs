//! The artifact sheets: the per-athlete best marks, the meet inventory, the evidence mix and the
//! method notes.
//!
//! These four sheets read the same census the scope sheets do and add nothing to it: the best-mark
//! rows come from the `bests` reduction, the rest are the census's own counters sorted for reading.

use crate::bests::BestResult;
use crate::report::{Census, ReportResult};

use super::cells::{cell, row, Cell};

pub(super) fn best_sheet(bests: &[BestResult]) -> ReportResult<Vec<Vec<Cell>>> {
    let mut rows = vec![row!(
        "Athlete",
        "School",
        "State",
        "Grad year",
        "Gender",
        "Sport",
        "Event",
        "Best mark",
        "Date",
        "Meet",
        "Place",
        "Wind m/s",
        "Timing",
        "Marks in event",
        "Athlete id",
        "Profile URL",
    )];
    for best in bests {
        rows.push(row!(
            Cell::text(best.name.clone()),
            Cell::text(best.school.clone()),
            Cell::text(best.state.clone()),
            Cell::Number(f64::from(best.grad_year)),
            Cell::text(best.gender.clone()),
            Cell::text(best.sport.clone()),
            Cell::text(best.event.clone()),
            Cell::text(best.best_mark.clone()),
            Cell::text(best.date.clone()),
            Cell::text(best.meet.clone()),
            best.place
                .map(|place| Cell::Number(f64::from(place)))
                .unwrap_or(Cell::Empty),
            best.wind_mps.map(Cell::Number).unwrap_or(Cell::Empty),
            Cell::text(best.timing.clone().unwrap_or_default()),
            Cell::number(best.marks_in_event)?,
            Cell::text(best.athlete_id.clone()),
            Cell::text(best.profile_url.clone().unwrap_or_default()),
        ));
    }
    Ok(rows)
}

pub(super) fn meets_sheet(core: &Census, all_sources: &Census) -> ReportResult<Vec<Vec<Cell>>> {
    let mut rows = meets_totals(core, all_sources)?;
    rows.extend(meets_by_state(core, all_sources)?);
    rows.extend(meets_by_provider(core)?);
    Ok(rows)
}

/// The inventory header: both scopes' totals and the core scope's date range.
fn meets_totals(core: &Census, all_sources: &Census) -> ReportResult<Vec<Vec<Cell>>> {
    let mut rows = vec![row!(
        "Core meet inventory",
        "Meets",
        "All-source meet inventory",
        "Meets"
    )];
    rows.push(row!(
        "Total",
        Cell::number(core.meets.total)?,
        "Total",
        Cell::number(all_sources.meets.total)?,
    ));
    rows.push(row!(
        "With an Athletic.net meet id",
        Cell::number(core.meets.with_athletic_net_id)?,
        "With an Athletic.net meet id",
        Cell::number(all_sources.meets.with_athletic_net_id)?,
    ));
    if let (Some(first), Some(last)) = (&core.meets.first_date, &core.meets.last_date) {
        rows.push(row!(
            "Date range",
            Cell::text(format!("{first} .. {last}")),
            "Date range",
            Cell::text(
                all_sources
                    .meets
                    .first_date
                    .clone()
                    .zip(all_sources.meets.last_date.clone())
                    .map(|(a, b)| format!("{a} .. {b}"))
                    .unwrap_or_default(),
            ),
        ));
    }
    Ok(rows)
}

/// The per-state counts of both scopes, side by side.
fn meets_by_state(core: &Census, all_sources: &Census) -> ReportResult<Vec<Vec<Cell>>> {
    let core_states: Vec<(&String, &usize)> = sorted_counts(&core.meets.by_state);
    let all_states: Vec<(&String, &usize)> = sorted_counts(&all_sources.meets.by_state);
    let mut rows = vec![row!()];
    rows.push(row!(
        "By state (core)",
        "Meets",
        "By state (all sources)",
        "Meets"
    ));
    for index in 0..core_states.len().max(all_states.len()) {
        let left = core_states.get(index);
        let right = all_states.get(index);
        rows.push(row!(
            left.map(|(state, _)| Cell::text((*state).clone()))
                .unwrap_or(Cell::Empty),
            left.map(|(_, count)| Cell::number(**count))
                .transpose()?
                .unwrap_or(Cell::Empty),
            right
                .map(|(state, _)| Cell::text((*state).clone()))
                .unwrap_or(Cell::Empty),
            right
                .map(|(_, count)| Cell::number(**count))
                .transpose()?
                .unwrap_or(Cell::Empty),
        ));
    }
    Ok(rows)
}

/// The core scope's counts by provider key.
fn meets_by_provider(core: &Census) -> ReportResult<Vec<Vec<Cell>>> {
    let providers: Vec<(&String, &usize)> = sorted_counts(&core.meets.by_provider);
    let mut rows = vec![row!()];
    rows.push(row!("By provider key (core)", "Meets", "", ""));
    for (provider, count) in providers {
        rows.push(row!(
            Cell::text(provider.clone()),
            Cell::number(*count)?,
            Cell::Empty,
            Cell::Empty,
        ));
    }
    Ok(rows)
}

pub(super) fn evidence_sheet(census: &Census) -> ReportResult<Vec<Vec<Cell>>> {
    let mut rows = Vec::new();
    let section = |rows: &mut Vec<Vec<Cell>>,
                   title: &str,
                   counts: &std::collections::BTreeMap<String, usize>|
     -> ReportResult<()> {
        rows.push(row!(Cell::text(title), Cell::text("Count")));
        for (key, value) in sorted_counts(counts) {
            rows.push(row!(Cell::text(key.clone()), Cell::number(*value)?,));
        }
        rows.push(row!());
        Ok(())
    };
    section(&mut rows, "Coach sources", &census.coach_sources)?;
    section(&mut rows, "Coach roles", &census.coach_roles)?;
    section(&mut rows, "Coach sports", &census.coach_sports)?;
    section(
        &mut rows,
        "Grade-evidence sources",
        &census.providers.grade_evidence_sources,
    )?;
    section(
        &mut rows,
        "Source namespaces on class-of-2027 athletes",
        &census.providers.namespaces,
    )?;
    section(
        &mut rows,
        "Athletes by graduating class",
        &census.athletes_by_grad_year,
    )?;
    rows.push(row!("Class-of-2027 sport mix", "Athletes"));
    let sports = &census.class_of_2027_sports;
    for (label, value) in [
        ("indoor only", sports.indoor_only),
        ("outdoor only", sports.outdoor_only),
        ("cross-country only", sports.cross_country_only),
        ("multi-sport", sports.multi_sport),
        ("no sport recorded", sports.none),
    ] {
        rows.push(row!(Cell::text(label), Cell::number(value)?));
    }
    Ok(rows)
}

pub(super) fn method_sheet() -> Vec<Vec<Cell>> {
    vec![
        row!("Result-artifact parsing", "96 parsed artifacts before the vendor layouts landed; 1,740 after (Compiled 763, cross-country 380, Hy-Tek 597); 834,254 result rows, 264,167 grade-bearing."),
        row!("Class-of-2027 evidence", "The artifact corpus added 4,323 Wisconsin class-of-2027 athletes to core (14,958 to 19,281)."),
        row!("North Dakota and South Dakota", "349 teams collected, 29,746 athletes, 4,712 class-of-2027, no errors."),
        row!("Wayzata schedules", "537 competition rows over two 2026 schedules minted 536 core meets; 304 rows resolved to a state (95 recurring sites, 209 schools)."),
        row!("Unresolved venues", "Filed under ?? rather than guessed; they are meet inventory, not athlete evidence."),
        row!("Best marks", "One row per (athlete, event): the winning mark on that event's own scale, with the meet, date, place, wind and timing that produced it. Relay legs are excluded - a squad mark is not a personal best."),
        row!("Coach coverage", "WI, MN, IL, OH, NE and ND publish directories; MI, MO, IN and KS still have none."),
    ]
}

fn sorted_counts(counts: &std::collections::BTreeMap<String, usize>) -> Vec<(&String, &usize)> {
    let mut ordered: Vec<(&String, &usize)> = counts.iter().collect();
    ordered.sort_by(|left, right| right.1.cmp(left.1).then_with(|| left.0.cmp(right.0)));
    ordered
}
