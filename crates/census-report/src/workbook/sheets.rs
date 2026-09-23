//! The scope sheets: the goal and method notes, the summary, both per-state views and what
//! Athletic.net alone still contributes.
//!
//! Every row is a column header or a census count copied out of `Census`/`StateCensus`; the counter
//! tables are the only thing that decides which field backs a printed column.

use crate::report::{Census, ReportError, ReportResult, StateCensus};
use census_domain::JurisdictionBucket;

use super::cells::{cell, row, share, Cell};

pub(super) fn goal_sheet(core: &Census, grad_year: Option<i16>) -> Vec<Vec<Cell>> {
    let cohort = grad_year
        .map(|year| format!("Class of {year}"))
        .unwrap_or_else(|| "every athlete in scope".to_string());
    vec![
        row!("Midwest high-school track & field / cross-country recruiting census"),
        row!(),
        row!("Goal"),
        row!("", "Own the canonical graph (school, team, coach, athlete, meet, event, performance) for boys and girls track & field and cross-country, keeping graduating class separate from the grade a source happened to publish, without depending on Athletic.net."),
        row!("Success test", "Athletic.net and its AthleticLIVE mirror switched off: the census still runs and reports."),
        row!("Core scope", "Evidence produced by adapters that do not read Athletic.net or its mirror."),
        row!("All-sources scope", "Adds the two AthleticLIVE modules (mirror + athlete index) as the comparison baseline."),
        row!(),
        row!("Cohort in this workbook", cohort),
        row!(),
        row!("Source tiers"),
        row!("Tier A", "Official state associations: WIAA, MSHSL, IHSA, OHSAA, KSHSAA, NDHSAA, NSAA"),
        row!("Tier B", "MileSplit-style state sites: rosters, graded athletes, public profile URLs"),
        row!("Tier D", "Official result artifacts: Hy-Tek, Compiled, cross-country and RaceDay layouts"),
        row!("Tier E", "Compliant timing providers: Wayzata Results (MN / IA / WI) published schedules"),
        row!("Non-core", "Athletic.net and the AthleticLIVE mirror: never fetched by a core run"),
        row!(),
        row!("Reproduce"),
        row!("Consolidate", "cargo run --release -p census-service -- consolidate"),
        row!("Core report", "cargo run --release -p census-service -- report --core"),
        row!("All-source report", "cargo run --release -p census-service -- report"),
        row!("Best marks", "cargo run --release -p census-service -- bests"),
        row!("Workbook", "cargo run --release -p census-service -- workbook"),
        row!(),
        row!("Provenance"),
        row!("Core report generated", core.generated_on.clone()),
        row!("Store", core.store_dir.clone()),
        row!("Core note", core.notes.first().cloned().unwrap_or_default()),
    ]
}

/// One census row: the label a sheet prints and the counted field behind it.
type StateCounter = (&'static str, fn(&StateCensus) -> usize);

/// The same shape over the whole census rather than one state.
type CensusCounter = (&'static str, fn(&Census) -> usize);

const SUMMARY_ROWS: [StateCounter; 12] = [
    ("Athletes (all grades)", |row| row.athletes),
    ("Class of 2027", |row| row.class_of_2027),
    ("Class of 2027 boys", |row| row.class_of_2027_boys),
    ("Class of 2027 girls", |row| row.class_of_2027_girls),
    ("Class of 2027, grade-evidenced", |row| {
        row.class_of_2027_with_grad_year_evidence
    }),
    ("Class of 2027, public profile URL", |row| {
        row.class_of_2027_with_profile_url
    }),
    ("Class of 2027, multiple sources", |row| {
        row.class_of_2027_multisource
    }),
    ("Class of 2027, coach identified", |row| {
        row.class_of_2027_with_coach
    }),
    ("Class of 2027, coach professional email", |row| {
        row.class_of_2027_with_coach_email
    }),
    ("Coaches", |row| row.coaches),
    ("Coaches with professional email", |row| {
        row.coaches_with_email
    }),
    ("Schools", |row| row.schools),
];

pub(super) fn summary_sheet(core: &Census, all_sources: &Census) -> ReportResult<Vec<Vec<Cell>>> {
    let mut rows = vec![row!(
        "Metric",
        "Core (Athletic.net off)",
        "All sources",
        "Core share"
    )];
    for (label, pick) in SUMMARY_ROWS {
        let core_value = pick(&core.totals);
        let all_value = pick(&all_sources.totals);
        rows.push(row!(
            Cell::text(label),
            Cell::number(core_value)?,
            Cell::number(all_value)?,
            share(core_value, all_value)?,
        ));
    }
    rows.push(row!());
    let enrichment_rows: [CensusCounter; 2] = [
        ("Core meets", |census: &Census| census.meets.total),
        ("Meets carrying an Athletic.net id", |census: &Census| {
            census.meets.with_athletic_net_id
        }),
    ];
    for (label, pick) in enrichment_rows {
        let core_value = pick(core);
        let all_value = pick(all_sources);
        rows.push(row!(
            Cell::text(label),
            Cell::number(core_value)?,
            Cell::number(all_value)?,
            if label.starts_with("Core meets") {
                share(core_value, all_value)?
            } else {
                Cell::text("enrichment key only, never dereferenced")
            },
        ));
    }
    rows.push(row!());
    rows.push(row!(
        "Athletes whose Athletic.net profile URL is known without an Athletic.net request",
    ));
    rows.push(row!(
        "",
        Cell::number(all_sources.providers.athletic_net_urls_known)?,
    ));
    Ok(rows)
}

const STATE_COLUMNS: [StateCounter; 12] = [
    ("Schools", |row| row.schools),
    ("Athletes", |row| row.athletes),
    ("Class of 2027", |row| row.class_of_2027),
    ("Co27 boys", |row| row.class_of_2027_boys),
    ("Co27 girls", |row| row.class_of_2027_girls),
    ("Co27 grade-evidenced", |row| {
        row.class_of_2027_with_grad_year_evidence
    }),
    ("Co27 profile URL", |row| row.class_of_2027_with_profile_url),
    ("Co27 multi-source", |row| row.class_of_2027_multisource),
    ("Co27 with coach", |row| row.class_of_2027_with_coach),
    ("Co27 with coach email", |row| {
        row.class_of_2027_with_coach_email
    }),
    ("Coaches", |row| row.coaches),
    ("Coaches with email", |row| row.coaches_with_email),
];

pub(super) fn state_sheet(census: &Census) -> ReportResult<Vec<Vec<Cell>>> {
    let mut header = vec![Cell::text("State")];
    header.extend(STATE_COLUMNS.iter().map(|(label, _)| Cell::text(*label)));
    header.extend(
        [
            "Co27 profile URL %",
            "Co27 with coach %",
            "Co27 with coach email %",
        ]
        .into_iter()
        .map(Cell::text),
    );
    let mut rows = vec![header];

    let mut ordered: Vec<(&JurisdictionBucket, &StateCensus)> = census.by_state.iter().collect();
    // Ties keep the printed label ascending: the published sheets ordered zero-count rows that way
    // when the bucket was still a string, and a sheet that reorders them is a different sheet.
    ordered.sort_by_key(|(state, row)| (std::cmp::Reverse(row.class_of_2027), state.code()));
    for (state, row) in ordered {
        rows.push(state_row(state.code(), row)?);
    }
    let total = state_row("TOTAL", &census.totals)?;
    rows.push(total);
    Ok(rows)
}

fn state_row(state: &str, row: &StateCensus) -> ReportResult<Vec<Cell>> {
    let mut cells = vec![Cell::text(state)];
    for (_, pick) in STATE_COLUMNS {
        cells.push(Cell::number(pick(row))?);
    }
    cells.push(share(
        row.class_of_2027_with_profile_url,
        row.class_of_2027,
    )?);
    cells.push(share(row.class_of_2027_with_coach, row.class_of_2027)?);
    cells.push(share(
        row.class_of_2027_with_coach_email,
        row.class_of_2027,
    )?);
    Ok(cells)
}

/// A running total for the marginal sheet: counts never wrap, they fail instead.
fn add_count(total: usize, count: usize) -> ReportResult<usize> {
    total.checked_add(count).ok_or(ReportError::CounterOverflow)
}

/// What the all-sources scope reports but the core scope does not: `all_sources - core`.
fn marginal(all_sources: usize, core: usize) -> ReportResult<usize> {
    all_sources
        .checked_sub(core)
        .ok_or_else(|| ReportError::Invariant {
            detail: "the core scope reports more than the all-sources scope".to_string(),
        })
}

pub(super) fn marginal_sheet(core: &Census, all_sources: &Census) -> ReportResult<Vec<Vec<Cell>>> {
    let mut ordered: Vec<(&JurisdictionBucket, &StateCensus)> =
        all_sources.by_state.iter().collect();
    ordered.sort_by_key(|(state, row)| (std::cmp::Reverse(row.class_of_2027), state.code()));
    let mut rows = vec![row!(
        "State",
        "Co27, all sources",
        "Co27, core",
        "Co27 only visible through Athletic.net",
        "Core share",
        "Athletes, all sources",
        "Athletes, core",
    )];
    let (mut all_total, mut core_total, mut athletes_all, mut athletes_core) = (0, 0, 0, 0);
    let empty = StateCensus {
        state: JurisdictionBucket::Unplaced.into(),
        schools: 0,
        athletes: 0,
        class_of_2027: 0,
        class_of_2027_boys: 0,
        class_of_2027_girls: 0,
        class_of_2027_unknown_gender: 0,
        class_of_2027_with_profile_url: 0,
        class_of_2027_with_grad_year_evidence: 0,
        class_of_2027_multisource: 0,
        class_of_2027_with_coach: 0,
        class_of_2027_with_coach_email: 0,
        coaches: 0,
        coaches_with_email: 0,
    };
    for (state, row) in ordered {
        let core_row = core.by_state.get(state).unwrap_or(&empty);
        all_total = add_count(all_total, row.class_of_2027)?;
        core_total = add_count(core_total, core_row.class_of_2027)?;
        athletes_all = add_count(athletes_all, row.athletes)?;
        athletes_core = add_count(athletes_core, core_row.athletes)?;
        rows.push(row!(
            Cell::text(state.code()),
            Cell::number(row.class_of_2027)?,
            Cell::number(core_row.class_of_2027)?,
            Cell::number(marginal(row.class_of_2027, core_row.class_of_2027)?)?,
            share(core_row.class_of_2027, row.class_of_2027)?,
            Cell::number(row.athletes)?,
            Cell::number(core_row.athletes)?,
        ));
    }
    rows.push(row!(
        Cell::text("TOTAL"),
        Cell::number(all_total)?,
        Cell::number(core_total)?,
        Cell::number(marginal(all_total, core_total)?)?,
        share(core_total, all_total)?,
        Cell::number(athletes_all)?,
        Cell::number(athletes_core)?,
    ));
    Ok(rows)
}
