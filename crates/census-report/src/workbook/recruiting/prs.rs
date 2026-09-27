use crate::bests::{Conflict, SharedSelection};
use crate::report::ReportResult;
use super::super::cells::{row, Cell};

pub(super) const TITLE: &str = "PRs";

pub(super) const HEADERS: [&str; 26] = [
    "Athlete ID", "Athlete", "Gender", "School", "State", "Graduation Year",
    "Sport", "Event", "Season", "Calculated PR", "Mark Value", "Unit", "Wind",
    "PR date", "Meet", "Place", "Result URL", "Source count", "PR conflict",
    "Surface", "Wind Class", "Timing Class", "Performance ID", "Meet ID",
    "Source Key", "Event Context",
];

pub(super) const WIDTHS: [u16; 26] = [
    20, 26, 10, 30, 8, 16, 12, 20, 14, 18, 14, 10, 12, 14, 40, 10, 44, 13, 30,
    14, 14, 14, 24, 24, 30, 24,
];

pub(super) fn sheet(prs: &[SharedSelection]) -> ReportResult<Vec<Vec<Cell>>> {
    let mut rows = vec![HEADERS.iter().map(|header| Cell::text(*header)).collect()];
    for pr in prs {
        rows.push(row!(
            Cell::text(pr.athlete_id().as_str()),
            Cell::text(&pr.athlete),
            Cell::text(pr.gender.stable_key()),
            pr.school.as_deref().map_or(Cell::Empty, Cell::text),
            Cell::text(pr.athlete_state.code()),
            Cell::Number(f64::from(pr.grad_year)),
            Cell::text(pr.sport()),
            Cell::text(pr.key.event_kind.stable_key()),
            Cell::text(pr.season()),
            Cell::text(pr.mark_text()),
            pr.normalized.map_or(Cell::Empty, Cell::Number),
            pr.unit().map_or(Cell::Empty, Cell::text),
            pr.wind_mps.map_or(Cell::Empty, Cell::Number),
            Cell::text(&pr.date),
            Cell::text(&pr.meet),
            pr.place.map_or(Cell::Empty, |place| Cell::Number(f64::from(place))),
            Cell::text(&pr.result_url),
            Cell::number(pr.population.sources)?,
            conflict_cell(&pr.conflicts),
            Cell::text(pr.key.surface.label()),
            Cell::text(pr.key.wind_class.label()),
            Cell::text(pr.key.timing.label()),
            Cell::text(pr.performance_id.as_str()),
            Cell::text(pr.meet_id.as_str()),
            Cell::text(&pr.source_key),
            pr.key.context.as_ref().map_or(Cell::Empty, |id| Cell::text(id.as_str())),
        ));
    }
    Ok(rows)
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
