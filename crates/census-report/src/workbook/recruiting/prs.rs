use super::super::cells::Cell;
use crate::bests::{Conflict, SharedSelection};
use crate::report::{ReportError, ReportResult};

pub(super) const TITLE: &str = "PRs";

pub(super) const HEADERS: [&str; 26] = [
    "Athlete ID",
    "Athlete",
    "Gender",
    "School",
    "State",
    "Graduation Year",
    "Sport",
    "Event",
    "Season",
    "Calculated PR",
    "Mark Value",
    "Unit",
    "Wind",
    "PR date",
    "Meet",
    "Place",
    "Result URL",
    "Source count",
    "PR conflict",
    "Surface",
    "Wind Class",
    "Timing Class",
    "Performance ID",
    "Meet ID",
    "Source Key",
    "Event Context",
];

pub(super) const WIDTHS: [u16; 26] = [
    20, 26, 10, 30, 8, 16, 12, 20, 14, 18, 14, 10, 12, 14, 40, 10, 44, 13, 30, 14, 14, 14, 24, 24,
    30, 24,
];

pub(super) fn sheet(prs: &[SharedSelection]) -> ReportResult<Vec<Vec<Cell>>> {
    let count = prs
        .len()
        .checked_add(1)
        .ok_or_else(|| ReportError::Invariant {
            detail: "PR sheet row count overflow".into(),
        })?;
    let mut rows = Vec::new();
    rows.try_reserve(count)
        .map_err(|error| ReportError::Invariant {
            detail: format!("reserving PR sheet rows: {error}"),
        })?;
    rows.push(HEADERS.iter().map(|header| Cell::text(*header)).collect());
    prs.iter().try_fold(rows, |mut rows, pr| {
        rows.push(cells(pr)?);
        Ok(rows)
    })
}

fn cells(pr: &SharedSelection) -> ReportResult<Vec<Cell>> {
    let mut cells = Vec::new();
    cells
        .try_reserve(HEADERS.len())
        .map_err(|error| ReportError::Invariant {
            detail: format!("reserving PR sheet cells: {error}"),
        })?;
    cells.extend(identity_cells(pr));
    cells.extend(result_cells(pr)?);
    cells.extend(context_cells(pr));
    Ok(cells)
}

fn identity_cells(pr: &SharedSelection) -> [Cell; 9] {
    [
        Cell::text(pr.athlete_id().as_str()),
        Cell::text(&pr.athlete.name),
        Cell::text(pr.athlete.gender.stable_key()),
        pr.athlete.school.as_deref().map_or(Cell::Empty, Cell::text),
        Cell::text(pr.athlete.athlete_state.code()),
        Cell::Number(f64::from(pr.athlete.grad_year)),
        Cell::text(pr.sport()),
        Cell::text(pr.key.event_kind.stable_key()),
        Cell::text(pr.season()),
    ]
}

fn result_cells(pr: &SharedSelection) -> ReportResult<[Cell; 10]> {
    Ok([
        Cell::text(pr.mark_text()),
        pr.result.normalized.map_or(Cell::Empty, Cell::Number),
        pr.unit().map_or(Cell::Empty, Cell::text),
        pr.result.wind_mps.map_or(Cell::Empty, Cell::Number),
        Cell::text(&pr.meet.date),
        Cell::text(&pr.meet.name),
        pr.result
            .place
            .map_or(Cell::Empty, |place| Cell::Number(f64::from(place))),
        Cell::text(&pr.source.result_url),
        Cell::number(pr.population.sources)?,
        conflict_cell(&pr.conflicts),
    ])
}

fn context_cells(pr: &SharedSelection) -> [Cell; 7] {
    [
        Cell::text(pr.key.surface.label()),
        Cell::text(pr.key.wind_class.label()),
        Cell::text(pr.key.timing.label()),
        Cell::text(pr.source.performance_id.as_str()),
        Cell::text(pr.meet.meet_id.as_str()),
        Cell::text(&pr.source.source_key),
        event_context(pr),
    ]
}

fn event_context(pr: &SharedSelection) -> Cell {
    match crate::bests::context::cross_country(pr) {
        Some(context) => Cell::text(context),
        None => pr
            .key
            .context
            .as_ref()
            .map_or(Cell::Empty, |id| Cell::text(id.as_str())),
    }
}

fn conflict_cell(conflicts: &[Conflict]) -> Cell {
    if conflicts.is_empty() {
        return Cell::Empty;
    }
    let mut text = String::new();
    for (index, conflict) in conflicts.iter().enumerate() {
        if index != 0 {
            text.push_str("; ");
        }
        text.push_str(&conflict.meet);
        text.push_str(": ");
        for (index, mark) in conflict.marks.iter().enumerate() {
            if index != 0 {
                text.push_str(" | ");
            }
            text.push_str(mark);
        }
    }
    Cell::text(text)
}
