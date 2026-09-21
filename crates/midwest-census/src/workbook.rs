//! The census as one spreadsheet.
//!
//! Nine sheets, every cell copied from the typed census the crate already computes: the two scope
//! reports, the marginal view of what Athletic.net alone still contributes, the per-athlete best
//! marks, the meet inventory, the evidence mix, and the method notes. Nothing is recomputed here, so
//! the workbook can never disagree with `report`.
//!
//! The workbook is written with the same `rust_xlsxwriter` dependency the rest of the workspace uses;
//! there is no external script in the loop. `bests::write` sidecars are emitted alongside it, so the
//! best-mark reduction is readable as text too.

use crate::bests::{self, BestResult};
use crate::report::{build_census, Census, Scope, StateCensus};
use crate::store::Store;
use anyhow::{Context, Result};
use rust_xlsxwriter::{Format, Workbook};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Options {
    pub grad_year: Option<i16>,
    pub out: Option<PathBuf>,
    pub limit: Option<usize>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            grad_year: Some(2027),
            out: None,
            limit: None,
        }
    }
}

/// One cell of a sheet.
#[derive(Debug, Clone)]
enum Cell {
    Text(String),
    Number(f64),
    Empty,
}

impl Cell {
    fn text(value: impl Into<String>) -> Self {
        Cell::Text(value.into())
    }

    /// A census count as the number an Excel cell holds.
    fn number(value: usize) -> Result<Self> {
        Ok(Cell::Number(count_as_number(value)?))
    }
}

/// Excel cells are `f64`, which holds every census count up to `u32::MAX` exactly; a larger count
/// is an error rather than a silently rounded cell.
fn count_as_number(value: usize) -> Result<f64> {
    let value = u32::try_from(value).context("cell count does not fit u32")?;
    Ok(f64::from(value))
}

impl From<&str> for Cell {
    fn from(value: &str) -> Self {
        Cell::Text(value.to_string())
    }
}

impl From<String> for Cell {
    fn from(value: String) -> Self {
        Cell::Text(value)
    }
}

/// Convert any supported cell source into a [`Cell`].
fn cell(value: impl Into<Cell>) -> Cell {
    value.into()
}

/// Build one sheet row from heterogeneous values: `row!(Cell::text(label), count, Cell::Empty)`.
///
/// A plain function cannot do this: an array literal forces one element type, and real rows mix
/// borrowed labels, owned strings, numbers, and explicit blanks.
macro_rules! row {
    () => { Vec::new() };
    ($($value:expr),+ $(,)?) => { vec![$(cell($value)),+] };
}

/// Build the workbook and its text sidecars; returns the path of the `.xlsx`.
pub fn build(store: &Store, options: &Options) -> Result<PathBuf> {
    let core = build_census(store, Scope::Core)?;
    let all_sources = build_census(store, Scope::AllSources)?;
    let bests = bests::build(
        store,
        &bests::Options {
            scope: Scope::Core,
            grad_year: options.grad_year,
            limit: options.limit,
        },
    )?;
    let cohort = options
        .grad_year
        .map(|year| format!("co{year}"))
        .unwrap_or_else(|| "all".to_string());
    bests::write(store, &bests, &cohort)?;

    let path = options.out.clone().unwrap_or_else(|| {
        store
            .out_dir()
            .join(format!("midwest-census-{}.xlsx", core.generated_on))
    });
    write_workbook(&path, &core, &all_sources, &bests, options.grad_year)?;
    Ok(path)
}

fn write_workbook(
    path: &Path,
    core: &Census,
    all_sources: &Census,
    bests: &[BestResult],
    grad_year: Option<i16>,
) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut book = Workbook::new();

    write_sheet(
        &mut book,
        "Goal & method",
        goal_sheet(core, grad_year),
        &[12, 96, 18, 12],
        false,
    )?;
    write_sheet(
        &mut book,
        "Summary",
        summary_sheet(core, all_sources)?,
        &[38, 22, 16, 14],
        false,
    )?;
    write_sheet(
        &mut book,
        "By state - core",
        state_sheet(core)?,
        &[
            10, 10, 12, 14, 11, 11, 18, 15, 14, 14, 17, 12, 16, 15, 17, 19,
        ],
        false,
    )?;
    write_sheet(
        &mut book,
        "By state - all sources",
        state_sheet(all_sources)?,
        &[
            10, 10, 12, 14, 11, 11, 18, 15, 14, 14, 17, 12, 16, 15, 17, 19,
        ],
        false,
    )?;
    write_sheet(
        &mut book,
        "Athletic.net marginal",
        marginal_sheet(core, all_sources)?,
        &[10, 16, 12, 30, 12, 16, 14],
        false,
    )?;
    write_sheet(
        &mut book,
        "Best results",
        best_sheet(bests)?,
        &[
            26, 14, 10, 9, 12, 10, 14, 16, 12, 13, 14, 10, 12, 11, 14, 26,
        ],
        true,
    )?;
    write_sheet(
        &mut book,
        "Meets",
        meets_sheet(core, all_sources)?,
        &[34, 12, 34, 12],
        false,
    )?;
    write_sheet(
        &mut book,
        "Evidence mix",
        evidence_sheet(all_sources)?,
        &[40, 12, 40, 12],
        false,
    )?;
    write_sheet(&mut book, "Method notes", method_sheet(), &[30, 110], false)?;

    book.save(path)
        .with_context(|| format!("saving the workbook to {}", path.display()))?;
    Ok(())
}

fn write_sheet(
    book: &mut Workbook,
    name: &str,
    rows: Vec<Vec<Cell>>,
    widths: &[u16],
    autofilter: bool,
) -> Result<()> {
    let bold = Format::new().set_bold();
    let sheet = book.add_worksheet();
    sheet.set_name(name)?;
    for (index, width) in widths.iter().enumerate() {
        let column = u16::try_from(index).context("column index does not fit u16")?;
        sheet.set_column_width(column, f64::from(*width))?;
    }
    let last_column = rows
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(1)
        .saturating_sub(1);
    for (row_index, cells) in rows.iter().enumerate() {
        let row_index = u32::try_from(row_index).context("row index does not fit u32")?;
        for (column, cell) in cells.iter().enumerate() {
            let column = u16::try_from(column).context("column index does not fit u16")?;
            match cell {
                Cell::Text(value) => {
                    if row_index == 0 {
                        sheet.write_string_with_format(row_index, column, value, &bold)?;
                    } else {
                        sheet.write_string(row_index, column, value)?;
                    }
                }
                Cell::Number(value) => {
                    sheet.write_number(row_index, column, *value)?;
                }
                Cell::Empty => {}
            }
        }
    }
    if autofilter && !rows.is_empty() {
        let last_row =
            u32::try_from(rows.len().saturating_sub(1)).context("row index does not fit u32")?;
        let last_column = u16::try_from(last_column).context("column index does not fit u16")?;
        sheet.autofilter(0, 0, last_row, last_column)?;
    }
    sheet.set_freeze_panes(1, 0)?;
    Ok(())
}

fn share(part: usize, whole: usize) -> Result<Cell> {
    if whole == 0 {
        Ok(Cell::text("n/a"))
    } else {
        Ok(Cell::text(format!(
            "{:.1}%",
            100.0 * count_as_number(part)? / count_as_number(whole)?
        )))
    }
}

fn goal_sheet(core: &Census, grad_year: Option<i16>) -> Vec<Vec<Cell>> {
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
        row!("Consolidate", "cargo run --release -p midwest-census -- consolidate"),
        row!("Core report", "cargo run --release -p midwest-census -- report --core"),
        row!("All-source report", "cargo run --release -p midwest-census -- report"),
        row!("Best marks", "cargo run --release -p midwest-census -- bests"),
        row!("Workbook", "cargo run --release -p midwest-census -- workbook"),
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

fn summary_sheet(core: &Census, all_sources: &Census) -> Result<Vec<Vec<Cell>>> {
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

fn state_sheet(census: &Census) -> Result<Vec<Vec<Cell>>> {
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

    let mut ordered: Vec<(&String, &StateCensus)> = census.by_state.iter().collect();
    ordered.sort_by_key(|(_, row)| std::cmp::Reverse(row.class_of_2027));
    for (state, row) in ordered {
        rows.push(state_row(state, row)?);
    }
    let total = state_row("TOTAL", &census.totals)?;
    rows.push(total);
    Ok(rows)
}

fn state_row(state: &str, row: &StateCensus) -> Result<Vec<Cell>> {
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
fn add_count(total: usize, count: usize) -> Result<usize> {
    total
        .checked_add(count)
        .context("the marginal totals do not fit usize")
}

/// What the all-sources scope reports but the core scope does not: `all_sources - core`.
fn marginal(all_sources: usize, core: usize) -> Result<usize> {
    all_sources
        .checked_sub(core)
        .context("the core scope reports more than the all-sources scope")
}

fn marginal_sheet(core: &Census, all_sources: &Census) -> Result<Vec<Vec<Cell>>> {
    let mut ordered: Vec<(&String, &StateCensus)> = all_sources.by_state.iter().collect();
    ordered.sort_by_key(|(_, row)| std::cmp::Reverse(row.class_of_2027));
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
        state: String::new(),
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
            Cell::text(state.clone()),
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

fn best_sheet(bests: &[BestResult]) -> Result<Vec<Vec<Cell>>> {
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

fn meets_sheet(core: &Census, all_sources: &Census) -> Result<Vec<Vec<Cell>>> {
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

    let core_states: Vec<(&String, &usize)> = sorted_counts(&core.meets.by_state);
    let all_states: Vec<(&String, &usize)> = sorted_counts(&all_sources.meets.by_state);
    rows.push(row!());
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

    let providers: Vec<(&String, &usize)> = sorted_counts(&core.meets.by_provider);
    rows.push(row!());
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

fn evidence_sheet(census: &Census) -> Result<Vec<Vec<Cell>>> {
    let mut rows = Vec::new();
    let section = |rows: &mut Vec<Vec<Cell>>,
                   title: &str,
                   counts: &std::collections::BTreeMap<String, usize>|
     -> Result<()> {
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

fn method_sheet() -> Vec<Vec<Cell>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{
        CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalSchool,
        CanonicalTeam, CompetitionLevel, EventKind, Evidence, Gender, GradYear, Mark, SchoolYear,
        SourceRef, Sport,
    };
    use crate::store::Table;
    use calamine::{open_workbook, Reader, Xlsx};

    #[test]
    fn the_workbook_carries_the_scopes_the_bests_and_the_meet_inventory() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::open(dir.path()).unwrap();
        let day = "2026-09-21";

        let (school, school_id) = CanonicalSchool::new("WI", "Abbotsford", "abbotsford");
        store.append(Table::Schools, &school).unwrap();

        let mut athlete = CanonicalAthlete::new(
            &school_id,
            "Julian Aguilera",
            GradYear::CO2027,
            Gender::Boys,
        );
        athlete
            .evidence
            .push(Evidence::parsed(SourceRef::new("wiaa_results", None), day));
        athlete
            .public_profile_urls
            .push("https://example.test/julian".to_string());
        store.append(Table::Athletes, &athlete).unwrap();

        let mut meet = CanonicalMeet::new(
            "WI",
            "WIAA Division 3 State",
            "2026-06-06",
            CompetitionLevel::State,
        );
        let meet_id = meet.id.clone();
        meet.evidence
            .push(Evidence::parsed(SourceRef::new("wiaa_results", None), day));
        store.append(Table::Meets, &meet).unwrap();
        let team_id = CanonicalTeam::mint(
            &school_id,
            Sport::OutdoorTrack,
            Gender::Boys,
            SchoolYear(2026),
        );

        for (kind, marks) in [
            (EventKind::Track400m, &[49.80_f64, 48.55, 49.10][..]),
            (EventKind::LongJump, &[6.10_f64, 6.42][..]),
        ] {
            let mut event = CanonicalEvent::new(&meet_id, kind.clone(), Gender::Boys, None, None);
            let event_id = event.id.clone();
            event
                .evidence
                .push(Evidence::parsed(SourceRef::new("wiaa_results", None), day));
            store.append(Table::Events, &event).unwrap();
            for mark in marks {
                let value = if matches!(kind, EventKind::Track400m) {
                    Mark::TimeSeconds(*mark)
                } else {
                    Mark::DistanceMetres(*mark)
                };
                let source_key = format!("test:{kind:?}:{mark}");
                let performance = CanonicalPerformance {
                    id: CanonicalPerformance::mint(&athlete.id, &meet_id, &kind, day, &source_key),
                    athlete: athlete.id.clone(),
                    team: team_id.clone(),
                    event: event_id.clone(),
                    meet: meet_id.clone(),
                    date: day.to_string(),
                    mark: value,
                    wind_mps: None,
                    place: None,
                    heat: None,
                    round: None,
                    timing: None,
                    observed_grade: None,
                    evidence: vec![Evidence::parsed(SourceRef::new("wiaa_results", None), day)],
                    source_key,
                };
                store.append(Table::Performances, &performance).unwrap();
            }
        }

        crate::census::consolidate(&store).unwrap();
        let options = Options::default();
        let path = build(&store, &options).unwrap();
        assert!(path.exists(), "the workbook exists at {}", path.display());

        let mut book: Xlsx<_> = open_workbook(&path).unwrap();
        let names = book.sheet_names().to_vec();
        for expected in [
            "Goal & method",
            "Summary",
            "By state - core",
            "By state - all sources",
            "Athletic.net marginal",
            "Best results",
            "Meets",
            "Evidence mix",
            "Method notes",
        ] {
            assert!(
                names.contains(&expected.to_string()),
                "missing sheet {expected}"
            );
        }

        // The best-mark reduction picked the fastest 400 and the longest jump.
        let bests = bests::build(
            &store,
            &bests::Options {
                scope: Scope::Core,
                grad_year: Some(2027),
                limit: None,
            },
        )
        .unwrap();
        let sprint = bests
            .iter()
            .find(|row| row.event.contains("400m"))
            .expect("a 400m best");
        assert_eq!(sprint.best_mark, "48.55");
        assert_eq!(sprint.marks_in_event, 3);
        let jump = bests
            .iter()
            .find(|row| row.event.contains("LongJump"))
            .expect("a long jump best");
        assert_eq!(jump.best_mark, "6.42 m");
        assert!(jump.place.is_none());

        let range = book.worksheet_range("Best results").unwrap();
        assert_eq!(
            range.get_value((0, 0)).map(|v| v.to_string()),
            Some("Athlete".to_string())
        );
        let header: Vec<String> = (0..16)
            .map(|col| {
                range
                    .get_value((0, col))
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            })
            .collect();
        assert!(header.contains(&"Best mark".to_string()));
        assert!(header.contains(&"Profile URL".to_string()));
    }
}
