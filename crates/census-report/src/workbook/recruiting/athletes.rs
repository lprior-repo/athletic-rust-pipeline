mod cells;
mod reported;
mod rules;

pub(crate) use rules::{HEADERS, TITLE, WIDTHS};

use super::super::cells::{row, Cell};
use super::columns::{coverage_state, flag, observed_school_year, published, source_count};
use super::contact::{self, Preferred, ScopedContacts};
use super::dataset::Dataset;
use super::facts::AthleteTally;
use super::profiles::profiles_of;
use crate::bests::SharedSelection;
use crate::report::ReportResult;
use crate::workbook::public_identity::{Identity, SHEET_HEADERS};
use cells::{
    event_list, gpa_source, headline_pr_summary, participation_flags, participation_metrics,
    pr_event_cells, profile_cells, public_recruiting_gpa, tf_flag,
};
use census_domain::model::CanonicalAthlete;

pub(super) fn rows<'a>(
    dataset: &'a Dataset<'a>,
) -> impl Iterator<Item = ReportResult<Vec<Cell>>> + 'a {
    let mut ordered: Vec<(&CanonicalAthlete, (String, String, String))> = dataset
        .athletes
        .iter()
        .map(|athlete| (athlete, display_key(dataset, athlete)))
        .collect();
    ordered.sort_by(|left, right| left.1.cmp(&right.1));
    let header = HEADERS
        .iter()
        .copied()
        .chain(
            dataset
                .public_identities
                .into_iter()
                .flat_map(|_| SHEET_HEADERS),
        )
        .map(|header| {
            let header = match (dataset.public_identities, header) {
                (Some(_), "Conflict Flag") => "Retained Source Conflict",
                (Some(_), "Review Status") => "Identity Authority",
                _ => header,
            };
            Cell::text(header)
        })
        .collect();
    std::iter::once(Ok(header)).chain(
        ordered
            .into_iter()
            .map(move |(athlete, _)| row_for(dataset, athlete)),
    )
}

pub(super) fn rows_with_reported<'a>(
    dataset: &'a Dataset<'a>,
    projection: Option<&'a crate::workbook::reported::Projection<'a>>,
) -> Box<dyn Iterator<Item = ReportResult<Vec<Cell>>> + 'a> {
    match projection {
        None => Box::new(rows(dataset)),
        Some(projection) => Box::new(
            rows(dataset)
                .enumerate()
                .map(|(index, row)| reported::verified(row, index == 0))
                .chain(
                    projection
                        .athletes
                        .iter()
                        .map(move |athlete| reported::row(dataset, athlete, projection)),
                ),
        ),
    }
}

pub(super) fn display_key(
    dataset: &Dataset<'_>,
    athlete: &CanonicalAthlete,
) -> (String, String, String) {
    let school = athlete.school.as_str();
    (
        dataset.school_state(school),
        dataset.school_name(school),
        athlete.canonical_name.clone(),
    )
}

fn row_for(dataset: &Dataset<'_>, athlete: &CanonicalAthlete) -> ReportResult<Vec<Cell>> {
    let school = athlete.school.as_str();
    let tally = dataset.tallies.get(athlete.id.as_str());
    let prs: Vec<&SharedSelection> = dataset.prs_of(athlete.id.as_str()).collect();
    let contacts = contact::scoped(dataset.contacts.get(school), athlete);
    let identity = Identity::of(dataset.public_identities, athlete.id.as_str())?;
    let mut cells = identity_cells(dataset, athlete, identity.athlete_id());
    append_performance_cells(&mut cells, athlete, &prs, tally)?;
    append_staff_cells(&mut cells, &contacts);
    cells.extend(school_athletics_url(dataset, school));
    cells.extend(public_recruiting_gpa());
    cells.extend(gpa_source());
    cells.extend(contact_cells(&contacts, contacts.preferred()));
    cells.extend(profile_cells(profiles_of(athlete)));
    cells.extend(audit_cells(dataset, athlete, tally, &prs, &identity)?);
    cells.push(address_cell(dataset, athlete)?);
    cells.extend(identity.audit()?.map(Cell::text));
    Ok(cells)
}

fn append_performance_cells(
    cells: &mut Vec<Cell>,
    athlete: &CanonicalAthlete,
    prs: &[&SharedSelection],
    tally: Option<&AthleteTally>,
) -> ReportResult<()> {
    cells.extend(tf_flag(athlete));
    cells.extend(participation_flags(athlete));
    cells.extend(event_list(athlete, prs));
    cells.extend(headline_pr_summary(athlete, prs));
    cells.extend(pr_event_cells(prs));
    cells.extend(participation_metrics(tally)?);
    Ok(())
}

fn append_staff_cells(cells: &mut Vec<Cell>, contacts: &ScopedContacts<'_>) {
    let director = contacts.director();
    cells.push(published(contacts.track_names()));
    cells.push(published(contacts.track_emails()));
    cells.push(published(
        contacts.cross_country().map(|coach| coach.name.clone()),
    ));
    cells.push(published(
        contacts
            .cross_country()
            .and_then(|coach| coach.address())
            .map(str::to_owned),
    ));
    cells.push(published(
        contacts.professional_coach_email().map(str::to_owned),
    ));
    cells.push(published(director.map(|d| d.name.clone())));
    cells.push(published(director.and_then(|d| d.email.clone())));
}

fn address_cell(dataset: &Dataset<'_>, athlete: &CanonicalAthlete) -> ReportResult<Cell> {
    match dataset.school_address.get(athlete.id.as_str()) {
        Some(address) if address.is_empty() => Ok(Cell::Empty),
        Some(address) => Ok(Cell::text(address.clone())),
        None => Err(crate::report::ReportError::Invariant {
            detail: format!("athlete {} has no school address projection", athlete.id),
        }),
    }
}

fn identity_cells(
    dataset: &Dataset<'_>,
    athlete: &CanonicalAthlete,
    athlete_id: &str,
) -> Vec<Cell> {
    let school = athlete.school.as_str();
    let school_id = dataset
        .schools
        .get(school)
        .map(|s| s.id.as_str())
        .map_or(school, |value| value)
        .to_string();
    row!(
        Cell::text(athlete_id),
        Cell::text(athlete.canonical_name.clone()),
        Cell::text(athlete.gender.stable_key().to_string()),
        Cell::Number(f64::from(athlete.grad_year.get())),
        observed_school_year(athlete),
        Cell::text(dataset.school_state(school)),
        Cell::text(dataset.school_name(school)),
        Cell::text(school_id),
        Cell::text(dataset.school_city(school)),
    )
}

fn school_athletics_url(dataset: &Dataset<'_>, school: &str) -> Vec<Cell> {
    let url = dataset
        .schools
        .get(school)
        .and_then(|s| s.athletics_website.clone());
    vec![published(url)]
}

fn audit_cells(
    dataset: &Dataset<'_>,
    athlete: &CanonicalAthlete,
    tally: Option<&AthleteTally>,
    prs: &[&SharedSelection],
    identity: &Identity<'_>,
) -> ReportResult<Vec<Cell>> {
    let historical_status = dataset
        .source
        .identities()
        .status(athlete.id.as_str())
        .map_err(census_store::StoreError::from)?;
    let (status, verification) = match identity {
        Identity::Selected(_) => ("accepted_public_identity", "retained_public_history"),
        Identity::Historical(_) => (
            historical_status.as_str(),
            verification_label(historical_status),
        ),
    };
    Ok(row!(
        Cell::number(source_count(athlete))?,
        Cell::text(status),
        Cell::text(coverage_state(
            tally.is_some_and(|t| t.performances > 0),
            !prs.is_empty()
        )),
        flag(historical_status == census_domain::model::IdentityStatus::RetainedConflict),
        Cell::text(verification),
    ))
}

fn verification_label(status: census_domain::model::IdentityStatus) -> &'static str {
    if status == census_domain::model::IdentityStatus::Verified {
        "verified"
    } else {
        "review"
    }
}

fn contact_cells(contacts: &ScopedContacts<'_>, preferred: Preferred<'_>) -> Vec<Cell> {
    row!(
        Cell::text(contacts.all_emails()),
        Cell::text(preferred.name),
        Cell::text(preferred.role),
        Cell::text(preferred.email),
        Cell::text(preferred.state.as_str()),
        Cell::text(preferred.coach_id),
        Cell::text(preferred.source_url),
        Cell::text(preferred.source_sha256),
        Cell::text(preferred.observed_on),
    )
}

#[cfg(test)]
mod tests;
