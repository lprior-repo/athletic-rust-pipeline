mod reported;

use super::super::cells::Cell;
use crate::bests::{Conflict, SharedSelection};
use crate::export::current_school::SelectedSchools;
use crate::export::public_identity::VerifiedPublicIdentityView;
use crate::report::{ReportError, ReportResult};
use crate::workbook::public_identity::{Identity, SHEET_HEADERS};

pub(super) const TITLE: &str = "PRs";

pub(super) const HEADERS: [&str; 29] = [
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
    "Published mark",
    "Conversion basis",
    "Conversion note",
];

pub(super) const WIDTHS: [u16; 29] = [
    20, 26, 10, 30, 8, 16, 12, 20, 14, 18, 14, 10, 12, 14, 40, 10, 44, 13, 30, 14, 14, 14, 24, 24,
    30, 24, 18, 16, 40,
];

pub(super) fn rows<'a>(
    prs: &'a [SharedSelection],
    view: Option<&'a VerifiedPublicIdentityView>,
    schools: Option<&'a SelectedSchools<'a>>,
) -> impl Iterator<Item = ReportResult<Vec<Cell>>> + 'a {
    let header = HEADERS
        .iter()
        .copied()
        .map(|name| match (schools.is_some(), name) {
            (true, "School") => "School at performance",
            (true, "State") => "State at performance",
            _ => name,
        })
        .chain(view.into_iter().flat_map(|_| SHEET_HEADERS))
        .chain(
            schools
                .into_iter()
                .flat_map(|_| ["Current school", "Current state", "Current school as-of"]),
        )
        .map(Cell::text)
        .collect();
    std::iter::once(Ok(header)).chain(prs.iter().map(move |pr| cells(pr, view, schools)))
}

pub(super) fn rows_with_reported<'a>(
    prs: &'a [SharedSelection],
    view: Option<&'a VerifiedPublicIdentityView>,
    schools: Option<&'a SelectedSchools<'a>>,
    projection: Option<&'a crate::workbook::reported::Projection<'a>>,
) -> Box<dyn Iterator<Item = ReportResult<Vec<Cell>>> + 'a> {
    match projection {
        None => Box::new(rows(prs, view, schools)),
        Some(projection) => Box::new(
            rows(prs, view, schools)
                .enumerate()
                .map(|(index, row)| reported::verified(row, index == 0))
                .chain(projection.bests.iter().map(move |reported| {
                    reported::row(reported, projection, view.is_some(), schools.is_some())
                })),
        ),
    }
}

fn cells(
    pr: &SharedSelection,
    view: Option<&VerifiedPublicIdentityView>,
    schools: Option<&SelectedSchools<'_>>,
) -> ReportResult<Vec<Cell>> {
    let identity = Identity::of(view, pr.athlete_id().as_str())?;
    let mut cells = Vec::new();
    cells
        .try_reserve(HEADERS.len())
        .map_err(|error| ReportError::Invariant {
            detail: format!("reserving PR sheet cells: {error}"),
        })?;
    cells.extend(identity_cells(pr, identity.athlete_id()));
    cells.extend(result_cells(pr)?);
    cells.extend(context_cells(pr));
    cells.extend(identity.audit()?.map(Cell::text));
    if let Some(schools) = schools {
        let current = schools.fields(pr.athlete_id().as_str())?;
        cells.extend(
            [
                current.current_school,
                current.current_state,
                current.current_school_as_of,
            ]
            .map(Cell::text),
        );
    }
    Ok(cells)
}

fn identity_cells(pr: &SharedSelection, athlete_id: &str) -> [Cell; 9] {
    [
        Cell::text(athlete_id),
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

fn context_cells(pr: &SharedSelection) -> [Cell; 10] {
    [
        Cell::text(pr.key.surface.label()),
        Cell::text(pr.key.wind_class.label()),
        Cell::text(pr.key.timing.label()),
        Cell::text(pr.source.performance_id.as_str()),
        Cell::text(pr.meet.meet_id.as_str()),
        Cell::text(&pr.source.source_key),
        event_context(pr),
        published_mark_cell(pr),
        conversion_basis_cell(pr),
        conversion_note_cell(pr),
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
fn published_mark_cell(pr: &SharedSelection) -> Cell {
    match &pr.result.original_mark {
        Some(original) => Cell::text(crate::bests::mark_text(original)),
        None => Cell::Empty,
    }
}

fn conversion_basis_cell(pr: &SharedSelection) -> Cell {
    match &pr.result.adjustment_kind {
        Some(_) => match pr.result.adjustment_basis {
            Some(basis) => Cell::text(basis.label()),
            None => Cell::text("unknown"),
        },
        None => Cell::Empty,
    }
}

fn conversion_note_cell(pr: &SharedSelection) -> Cell {
    match &pr.result.adjustment_note {
        Some(note) => Cell::text(note),
        None => Cell::Empty,
    }
}
