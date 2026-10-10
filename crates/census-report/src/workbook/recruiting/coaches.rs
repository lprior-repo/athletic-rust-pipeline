use super::super::cells::Cell;
use super::coach_projection;
use super::dataset::Dataset;
use crate::report::ReportResult;
use census_domain::model::{CanonicalCoach, SchoolYear};

pub(super) const TITLE: &str = "Coaches";
pub(super) const HEADERS: [&str; 29] = [
    "School ID",
    "School",
    "School City",
    "State",
    "Sport",
    "Coach",
    "Coach ID",
    "Gender",
    "Role",
    "Professional Email",
    "Personal Email",
    "Phone",
    "Athletic Director",
    "AD Professional Email",
    "Official Source URL",
    "Observed Date",
    "Declared Tenure",
    "Assessment School Year",
    "Professional Email Source URL",
    "Professional Email Capture SHA256",
    "Professional Email Acquired At",
    "Personal Email Source URL",
    "Personal Email Capture SHA256",
    "Personal Email Acquired At",
    "Name Capture SHA256",
    "AD Email Source URL",
    "AD Email Capture SHA256",
    "AD Email Acquired At",
    "Contact Admission State",
];
pub(super) const WIDTHS: [u16; 29] = [
    20, 30, 20, 8, 14, 26, 36, 10, 16, 32, 32, 18, 26, 32, 40, 28, 24, 22, 40, 64, 28, 40, 64, 28,
    64, 40, 64, 28, 28,
];
type SortKey = (String, String, String, String, String);

pub(super) fn sheet(dataset: &Dataset) -> ReportResult<Vec<Vec<Cell>>> {
    let names = &dataset.coach_spellings;
    let mut ordered: Vec<_> = dataset
        .published_coaches
        .iter()
        .map(|coach| (coach, sort_key(dataset, coach)))
        .collect();
    ordered.sort_by(|left, right| left.1.cmp(&right.1));
    let mut rows = vec![HEADERS.iter().map(|header| Cell::text(*header)).collect()];
    rows.extend(
        ordered
            .into_iter()
            .map(|(coach, _)| row_for(dataset, coach, names)),
    );
    Ok(rows)
}

fn sort_key(dataset: &Dataset, coach: &CanonicalCoach) -> SortKey {
    (
        dataset.school_state(coach.school.as_str()),
        dataset.school_name(coach.school.as_str()),
        sport_label(coach),
        coach.role.stable_key().to_owned(),
        coach.name.clone(),
    )
}

fn row_for(
    dataset: &Dataset,
    coach: &CanonicalCoach,
    names: &std::collections::BTreeMap<String, String>,
) -> Vec<Cell> {
    let contacts = dataset.contacts.get(coach.school.as_str());
    let selected = coach_projection::admitted(coach, dataset.school_year, contacts);
    let director = contacts.and_then(|contacts| contacts.director());
    let [_, name_digest, _] = coach_projection::capture(selected.name);
    let mut cells = identity_cells(
        dataset,
        coach,
        super::coach_spelling::published(names, coach),
    );
    cells.extend(contact_cells(
        coach,
        dataset.school_year,
        &selected,
        director,
    ));
    cells.extend(capture_cells(
        selected.professional.map(|contact| contact.tenure()),
    ));
    cells.extend(capture_cells(
        selected.personal.map(|contact| contact.tenure()),
    ));
    cells.push(Cell::text(name_digest));
    cells.extend(director_capture(director));
    cells.push(Cell::text(selected.state));
    cells
}

fn contact_cells(
    coach: &CanonicalCoach,
    year: SchoolYear,
    selected: &coach_projection::Projection<'_>,
    director: Option<&super::contact::Named>,
) -> [Cell; 9] {
    let [name_url, _, name_at] = coach_projection::capture(selected.name);
    [
        optional_text(selected.professional.map(|contact| contact.mailbox())),
        optional_text(selected.personal.map(|contact| contact.mailbox())),
        optional_text(coach.phone.as_deref()),
        optional_text(director.map(|director| director.name.as_str())),
        optional_text(director.and_then(|director| director.email.as_deref())),
        Cell::text(name_url),
        Cell::text(name_at),
        Cell::text(coach_projection::tenure_label(coach, year)),
        Cell::text(year.short()),
    ]
}

fn optional_text(value: Option<&str>) -> Cell {
    Cell::text(value.map_or("", core::convert::identity))
}

fn identity_cells(dataset: &Dataset, coach: &CanonicalCoach, name: &str) -> Vec<Cell> {
    [
        coach.school.to_string(),
        dataset.school_name(coach.school.as_str()),
        dataset.school_city(coach.school.as_str()),
        dataset.school_state(coach.school.as_str()),
        sport_label(coach),
        name.to_string(),
        coach.id.to_string(),
        coach.gender.stable_key().to_owned(),
        coach.role.stable_key().to_owned(),
    ]
    .into_iter()
    .map(Cell::text)
    .collect()
}

fn capture_cells(fact: Option<&census_domain::model::CoachTenureEvidence>) -> [Cell; 3] {
    coach_projection::capture(fact).map(Cell::text)
}

fn director_capture(director: Option<&super::contact::Named>) -> [Cell; 3] {
    let source = director.and_then(|director| director.professional_source());
    [
        Cell::text(source.map_or("", |source| source.source_url.as_str())),
        Cell::text(source.map_or("", |source| source.source_sha256.as_str())),
        Cell::text(source.map_or("", |source| source.observed_on.as_str())),
    ]
}

fn sport_label(coach: &CanonicalCoach) -> String {
    match (coach.sport, coach.role) {
        (Some(sport), _) => sport.stable_key().to_owned(),
        (None, census_domain::model::CoachRole::AthleticDirector) => "school_wide".to_owned(),
        (None, _) => "unknown".to_owned(),
    }
}
