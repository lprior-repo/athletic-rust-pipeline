//! The run-metrics sheet: what the store holds, what the run already fetched, both census scopes'
//! headline counters, and the block that reconciles every row-level sheet tally against the census
//! counter it must equal.
//!
//! The reconciliation is the point of the sheet. Each of the other meta sheets counts what its rows
//! are; the census document counts the same tables its own way. Both numbers are printed side by
//! side with a `reconciled`/`DIFFERS` status, so a workbook that disagrees with `report.json` names
//! the counter instead of leaving an operator to diff two artifacts. `DIFFERS` is reported, not
//! raised: the store is append-only and a collection process may write between the census and this
//! scan, which is exactly the drift a reader has to see.

use crate::bests::BestResult;
use crate::report::{io_error, Census, ReportResult};
use crate::store::Store;
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalMeet, GradYear, SourceNamespace,
};

use crate::workbook::cells::{cell, row, Cell};

use super::queues::SCHOOL_IDENTITY;
use super::{Family, StoreRows};

/// Widths for the metric blocks.
pub(super) const METRIC_WIDTHS: [u16; 4] = [40, 18, 18, 12];

/// The run's counters, in blocks: run identity, store counters, HTTP cache, both census scopes, and
/// the reconciliation of the row-level sheets against the census.
pub(super) fn metrics_sheet(
    store: &Store,
    core: &Census,
    all_sources: &Census,
    bests: &[BestResult],
    rows: &StoreRows,
    conflicts: &[Family],
) -> ReportResult<Vec<Vec<Cell>>> {
    let mut cells = vec![row!("Run metric", "Value")];
    cells.push(row!(
        "Workbook generated on",
        Cell::text(core.generated_on.clone())
    ));
    cells.push(row!("Store", Cell::text(core.store_dir.clone())));
    cells.push(row!("Census scopes published", "core + all sources"));
    cells.push(row!("Best-mark rows reduced", Cell::number(bests.len())?));
    cells.push(row!("Cohort behind the counters", "class of 2027"));
    cells.push(row!());
    cells.extend(store_counters(store)?);
    cells.push(row!());
    cells.extend(cache_block(store)?);
    cells.push(row!());
    cells.extend(scope_counters(core, all_sources)?);
    cells.push(row!());
    cells.extend(reconciliation(rows, conflicts, all_sources)?);
    Ok(cells)
}

/// The store's append counters: one row per table, then the totals the LSM tree reports.
fn store_counters(store: &Store) -> ReportResult<Vec<Vec<Cell>>> {
    let stats = store.stats()?;
    let mut cells = vec![row!("Store table", "Observations")];
    for (table, count) in &stats.tables {
        cells.push(row!(Cell::text(table.clone()), count_cell(*count)));
    }
    cells.push(row!("Total observations", count_cell(stats.observations)));
    cells.push(row!(
        "Store bytes on disk",
        Cell::text(stats.bytes_on_disk.to_string())
    ));
    Ok(cells)
}

/// What the HTTP cache holds: one entry per response already fetched, so a resumed run reports the
/// whole cache rather than only what this process fetched.
fn cache_block(store: &Store) -> ReportResult<Vec<Vec<Cell>>> {
    let dir = store.http_cache_dir();
    let entries = std::fs::read_dir(&dir).map_err(|source| io_error(&dir, source))?;
    let mut responses = 0_u64;
    let mut bytes = 0_u64;
    for entry in entries {
        let entry = entry.map_err(|source| io_error(&dir, source))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.ends_with(".meta.json") {
            responses = responses.saturating_add(1);
        } else if name.ends_with(".body") {
            let size = entry
                .metadata()
                .map_err(|source| io_error(&dir, source))?
                .len();
            bytes = bytes.saturating_add(size);
        }
    }
    Ok(vec![
        row!("HTTP cache", "Value"),
        row!("Cached responses", count_cell(responses)),
        row!("Cached bytes", Cell::text(bytes.to_string())),
    ])
}

/// One census counter: the label a block prints and the counted field behind it.
type ScopeCounter = (&'static str, fn(&Census) -> usize);

/// The counted fields of both census scopes, in published order.
const SCOPE_COUNTERS: [ScopeCounter; 7] = [
    ("Schools", |census| census.totals.schools),
    ("Athletes", |census| census.totals.athletes),
    ("Coaches", |census| census.totals.coaches),
    ("Coaches with a professional email", |census| {
        census.totals.coaches_with_email
    }),
    ("Class-of-2027 athletes", |census| {
        census.totals.class_of_2027
    }),
    ("Meets", |census| census.meets.total),
    ("Meets naming an Athletic.net id", |census| {
        census.meets.with_athletic_net_id
    }),
];

/// Both census scopes' headline counters, side by side.
fn scope_counters(core: &Census, all_sources: &Census) -> ReportResult<Vec<Vec<Cell>>> {
    let mut cells = vec![row!("Core", "Count", "All sources", "Count")];
    for (label, read) in SCOPE_COUNTERS {
        cells.push(row!(
            Cell::text(label),
            Cell::number(read(core))?,
            Cell::text(label),
            Cell::number(read(all_sources))?,
        ));
    }
    Ok(cells)
}

/// The reconciliation block: each row-level tally beside the census counter it must equal.
fn reconciliation(
    rows: &StoreRows,
    conflicts: &[Family],
    all_sources: &Census,
) -> ReportResult<Vec<Vec<Cell>>> {
    let mut cells = vec![row!("Reconciled counter", "Sheet rows", "Census", "Status")];
    let sheet_counts = [
        ("Schools", rows.schools.len(), all_sources.totals.schools),
        ("Meets", rows.meets.len(), all_sources.meets.total),
        ("Athletes", rows.athletes.len(), all_sources.totals.athletes),
        ("Coaches", rows.coaches.len(), all_sources.totals.coaches),
        (
            "Coaches with a professional email",
            count_coaches_with_email(&rows.coaches),
            all_sources.totals.coaches_with_email,
        ),
        (
            "Class-of-2027 athletes",
            count_co2027(&rows.athletes),
            all_sources.totals.class_of_2027,
        ),
        (
            "Class-of-2027 athletes with grade evidence",
            count_grade_evidence(&rows.athletes),
            all_sources.totals.class_of_2027_with_grad_year_evidence,
        ),
        (
            "Meets naming an Athletic.net id",
            count_athletic_net_meets(&rows.meets),
            all_sources.meets.with_athletic_net_id,
        ),
        (
            "Schools sharing a normalized name",
            findings_of(conflicts, SCHOOL_IDENTITY),
            all_sources.duplicate_school_names,
        ),
    ];
    for (label, sheet, census) in sheet_counts {
        cells.push(reconciled(label, sheet, census)?);
    }
    Ok(cells)
}

/// One reconciliation row: the sheet's tally beside the census counter it must equal.
fn reconciled(label: &str, sheet: usize, census: usize) -> ReportResult<Vec<Cell>> {
    let status = if sheet == census {
        "reconciled"
    } else {
        "DIFFERS"
    };
    Ok(row!(
        Cell::text(label),
        Cell::number(sheet)?,
        Cell::number(census)?,
        Cell::text(status)
    ))
}

/// How many findings one family of the queue holds.
fn findings_of(families: &[Family], label: &str) -> usize {
    families
        .iter()
        .find(|family| family.label == label)
        .map_or(0, |family| family.findings)
}

/// Coaches carrying a professional address.
fn count_coaches_with_email(coaches: &[CanonicalCoach]) -> usize {
    coaches
        .iter()
        .filter(|coach| coach.professional_email.is_some())
        .count()
}

/// The store's class-of-2027 athlete rows.
fn count_co2027(athletes: &[CanonicalAthlete]) -> usize {
    athletes
        .iter()
        .filter(|athlete| athlete.grad_year == GradYear::CO2027)
        .count()
}

/// Class-of-2027 athletes carrying at least one grade observation.
fn count_grade_evidence(athletes: &[CanonicalAthlete]) -> usize {
    athletes
        .iter()
        .filter(|athlete| {
            athlete.grad_year == GradYear::CO2027 && !athlete.observed_grades.is_empty()
        })
        .count()
}

/// Meets whose source identities include a legacy Athletic.net meet id, the predicate the census's
/// own `with_athletic_net_id` counter uses.
fn count_athletic_net_meets(meets: &[CanonicalMeet]) -> usize {
    meets
        .iter()
        .filter(|meet| {
            meet.source_identities.iter().any(|identity| {
                matches!(
                    identity.namespace,
                    SourceNamespace::LegacyAthleticNet { .. }
                )
            })
        })
        .count()
}

/// A counter as the number an Excel cell holds: exact through `u32`, and printed as text above it,
/// where a cell could no longer hold the value exactly.
fn count_cell(value: u64) -> Cell {
    match u32::try_from(value) {
        Ok(value) => Cell::Number(f64::from(value)),
        Err(_) => Cell::text(value.to_string()),
    }
}
