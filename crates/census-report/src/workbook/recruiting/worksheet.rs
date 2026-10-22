use super::{athletes, contact, dataset::Dataset, profiles::profiles_of};
use crate::export::current_school::SelectedSchools;
use crate::report::{ReportError, ReportResult};
use crate::workbook::cells::{write_sheet_rows, Cell, SheetChrome, SheetLayout};
use crate::workbook::public_identity::Identity;
use crate::workbook::reported::Projection;
use census_domain::model::CanonicalAthlete;
use rust_xlsxwriter::{Format, FormatAlign, Workbook};
use std::collections::BTreeSet;
use std::path::Path;

mod marks;
mod profile_prs;

#[cfg(test)]
mod tests;

const HEADERS: [&str; 24] = [
    "Athlete ID",
    "Name",
    "Gender",
    "Graduation Year",
    "Observed Grade",
    "Grade School Year",
    "School",
    "School ID",
    "School City",
    "School State",
    "School Address",
    "School Basis",
    "Current school",
    "Current state",
    "Current school as-of",
    "Head TF Coach",
    "Head XC Coach",
    "Observed bests",
    "Reported bests",
    "Unresolved results",
    "Source-declared PRs",
    "Source Profile",
    "Record Basis",
    "Verification Status",
];

const WIDTHS: [u16; 24] = [
    24, 28, 12, 18, 16, 18, 32, 24, 24, 14, 52, 30, 32, 14, 24, 28, 28, 80, 80, 64, 64, 64, 24, 44,
];

#[derive(Clone, Copy)]
enum Basis {
    Accepted,
    Reported,
}

pub(super) fn write(
    book: &mut Workbook,
    path: &Path,
    dataset: &Dataset<'_>,
    reported: Option<&Projection<'_>>,
) -> ReportResult<()> {
    let schools = SelectedSchools::of(dataset.source)?;
    let mut accepted = dataset.athletes.iter().collect::<Vec<_>>();
    accepted.sort_by_key(|athlete| athletes::display_key(dataset, athlete));
    let rows = std::iter::once(Ok(HEADERS.into_iter().map(Cell::text).collect()))
        .chain(
            accepted
                .into_iter()
                .map(|athlete| row(dataset, athlete, Basis::Accepted, reported, &schools)),
        )
        .chain(
            reported
                .into_iter()
                .flat_map(|projection| projection.athletes.iter())
                .map(|athlete| row(dataset, athlete, Basis::Reported, reported, &schools)),
        );
    write_sheet_rows(
        book,
        path,
        SheetLayout {
            name: "Athletes",
            widths: &WIDTHS,
            chrome: SheetChrome::Filtered,
        },
        rows,
    )?;
    let wrapped = Format::new().set_text_wrap().set_align(FormatAlign::Top);
    let sheet = book
        .worksheet_from_name("Athletes")
        .map_err(|error| crate::report::xlsx_error(path, error))?;
    for column in 17..=20 {
        sheet
            .set_column_format(column, &wrapped)
            .map_err(|error| crate::report::xlsx_error(path, error))?;
    }
    sheet
        .set_freeze_panes(1, 2)
        .map_err(|error| crate::report::xlsx_error(path, error))?;
    Ok(())
}

fn row(
    dataset: &Dataset<'_>,
    athlete: &CanonicalAthlete,
    basis: Basis,
    reported: Option<&Projection<'_>>,
    schools: &SelectedSchools<'_>,
) -> ReportResult<Vec<Cell>> {
    let mut cells = person(dataset, athlete, basis)?;
    cells.extend(school(dataset, athlete, basis)?);
    cells.extend(current_school(schools, athlete, basis)?);
    cells.extend(coaches(dataset, athlete, basis));
    cells.extend(marks::cells(dataset, athlete, reported, schools)?);
    cells.push(profile_prs::cell(dataset.source, athlete)?);
    cells.push(
        profiles_of(athlete)
            .milesplit
            .map_or(Cell::Empty, Cell::text),
    );
    cells.extend(match basis {
        Basis::Accepted => [
            Cell::text("verified"),
            Cell::text("verified_identity_and_affiliation"),
        ],
        Basis::Reported => [
            Cell::text("source_reported"),
            Cell::text("reported_unverified_needs_review"),
        ],
    });
    Ok(cells)
}

fn person(
    dataset: &Dataset<'_>,
    athlete: &CanonicalAthlete,
    basis: Basis,
) -> ReportResult<Vec<Cell>> {
    let id = match basis {
        Basis::Accepted => Identity::of(dataset.public_identities, athlete.id.as_str())?
            .athlete_id()
            .to_owned(),
        Basis::Reported => athlete.id.as_str().to_owned(),
    };
    let (grade, school_year) = observed_grade(athlete)?;
    Ok(vec![
        Cell::text(id),
        Cell::text(&athlete.canonical_name),
        Cell::text(athlete.gender.stable_key()),
        Cell::Number(f64::from(athlete.grad_year.get())),
        grade,
        school_year,
    ])
}

fn observed_grade(athlete: &CanonicalAthlete) -> ReportResult<(Cell, Cell)> {
    let Some(year) = athlete
        .observed_grades
        .iter()
        .map(|grade| grade.school_year)
        .max()
    else {
        return Ok((Cell::Empty, Cell::Empty));
    };
    let grades = athlete
        .observed_grades
        .iter()
        .filter(|grade| grade.school_year == year)
        .map(|grade| grade.grade.get())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|grade| grade.to_string())
        .collect::<Vec<_>>()
        .join(" | ");
    if grades.encode_utf16().count() > 32_767 {
        return Err(invalid("observed grade exceeds the Excel cell budget"));
    }
    Ok((Cell::text(grades), Cell::text(year.short())))
}

fn school(
    dataset: &Dataset<'_>,
    athlete: &CanonicalAthlete,
    basis: Basis,
) -> ReportResult<Vec<Cell>> {
    let school = dataset.source.schools.get(&athlete.school);
    let mut addresses = crate::export::postal::athlete_address_index(
        dataset.source,
        std::slice::from_ref(athlete),
    )?;
    let address = addresses
        .remove(athlete.id.as_str())
        .ok_or_else(|| invalid("worksheet school address is absent"))?;
    Ok(vec![
        school.map_or(Cell::Empty, |school| Cell::text(&school.name)),
        Cell::text(athlete.school.as_str()),
        school
            .and_then(|school| school.city.as_ref())
            .map_or(Cell::Empty, Cell::text),
        school
            .and_then(|school| school.state)
            .map_or(Cell::Empty, |state| Cell::text(state.code())),
        Cell::text(address),
        Cell::text(match basis {
            Basis::Accepted => "accepted_current_affiliation",
            Basis::Reported => "source_reported_affiliation",
        }),
    ])
}

fn current_school(
    schools: &SelectedSchools<'_>,
    athlete: &CanonicalAthlete,
    basis: Basis,
) -> ReportResult<[Cell; 3]> {
    match basis {
        Basis::Reported => Ok([Cell::text("Unknown"), Cell::text("Unknown"), Cell::Empty]),
        Basis::Accepted => {
            let fields = schools.fields(athlete.id.as_str())?;
            Ok([
                Cell::text(fields.current_school),
                Cell::text(fields.current_state),
                Cell::text(fields.current_school_as_of),
            ])
        }
    }
}

fn coaches(dataset: &Dataset<'_>, athlete: &CanonicalAthlete, basis: Basis) -> [Cell; 2] {
    match basis {
        Basis::Reported => [Cell::Empty, Cell::Empty],
        Basis::Accepted => {
            let scoped = contact::scoped(dataset.contacts.get(athlete.school.as_str()), athlete);
            [
                scoped.track_names().map_or(Cell::Empty, Cell::text),
                scoped
                    .cross_country()
                    .map_or(Cell::Empty, |coach| Cell::text(&coach.name)),
            ]
        }
    }
}

fn invalid(detail: &str) -> ReportError {
    ReportError::Invariant {
        detail: detail.to_owned(),
    }
}
