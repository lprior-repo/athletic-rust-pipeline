mod cells;
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
use cells::{
    event_list, gpa_source, headline_pr_summary, participation_flags, participation_metrics,
    pr_event_cells, profile_cells, public_recruiting_gpa, tf_flag,
};
use census_domain::model::CanonicalAthlete;

pub(super) fn sheet(dataset: &Dataset) -> ReportResult<Vec<Vec<Cell>>> {
    let mut ordered: Vec<(&CanonicalAthlete, (String, String, String))> = dataset
        .athletes
        .iter()
        .map(|athlete| {
            let school = athlete.school.as_str();
            (
                athlete,
                (
                    dataset.school_state(school),
                    dataset.school_name(school),
                    athlete.canonical_name.clone(),
                ),
            )
        })
        .collect();
    ordered.sort_by(|left, right| left.1.cmp(&right.1));
    let mut rows = vec![HEADERS.iter().map(|header| Cell::text(*header)).collect()];
    for (athlete, _) in ordered {
        rows.push(row_for(dataset, athlete)?);
    }
    Ok(rows)
}

fn row_for(dataset: &Dataset, athlete: &CanonicalAthlete) -> ReportResult<Vec<Cell>> {
    let school = athlete.school.as_str();
    let tally = dataset.tallies.get(athlete.id.as_str());
    let prs: Vec<&SharedSelection> = dataset.prs_of(athlete.id.as_str()).collect();
    let profiles = profiles_of(athlete);
    let contacts = contact::scoped(dataset.contacts.get(school), athlete);
    let preferred = contacts.preferred();
    let director = contacts.director();
    let mut cells = identity_cells(dataset, athlete);
    cells.extend(tf_flag(athlete));
    cells.extend(participation_flags(athlete));
    cells.extend(event_list(athlete, &prs));
    cells.extend(headline_pr_summary(athlete, &prs));
    cells.extend(pr_event_cells(&prs));
    cells.extend(participation_metrics(tally)?);
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
    cells.extend(school_athletics_url(dataset, school));
    cells.extend(public_recruiting_gpa());
    cells.extend(gpa_source());
    cells.extend(contact_cells(&contacts, preferred));
    cells.extend(profile_cells(profiles));
    cells.extend(audit_cells(dataset, athlete, tally, &prs)?);
    let postal = dataset.postal.get(athlete.id.as_str()).ok_or_else(|| {
        crate::report::ReportError::Invariant {
            detail: format!("athlete {} has no postal projection", athlete.id),
        }
    })?;
    cells.extend(postal.iter().cloned().map(Cell::text));
    Ok(cells)
}

fn identity_cells(dataset: &Dataset, athlete: &CanonicalAthlete) -> Vec<Cell> {
    let school = athlete.school.as_str();
    let school_id = dataset
        .schools
        .get(school)
        .map(|s| s.id.as_str())
        .unwrap_or(school)
        .to_string();
    row!(
        Cell::text(athlete.id.as_str()),
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

fn school_athletics_url(dataset: &Dataset, school: &str) -> Vec<Cell> {
    let url = dataset
        .schools
        .get(school)
        .and_then(|s| s.athletics_website.clone());
    vec![published(url)]
}

fn audit_cells(
    dataset: &Dataset,
    athlete: &CanonicalAthlete,
    tally: Option<&AthleteTally>,
    prs: &[&SharedSelection],
) -> ReportResult<Vec<Cell>> {
    let status = dataset
        .identities
        .status(athlete.id.as_str())
        .map_err(census_store::StoreError::from)?;
    Ok(row!(
        Cell::number(source_count(athlete))?,
        Cell::text(status.as_str()),
        Cell::text(coverage_state(
            tally.is_some_and(|t| t.performances > 0),
            !prs.is_empty()
        )),
        flag(status == census_domain::model::IdentityStatus::RetainedConflict),
        Cell::text(
            if status == census_domain::model::IdentityStatus::Verified {
                "verified"
            } else {
                "review"
            }
        ),
    ))
}

fn contact_cells(contacts: &ScopedContacts<'_>, preferred: Preferred) -> Vec<Cell> {
    row!(
        Cell::text(contacts.all_emails()),
        Cell::text(preferred.name),
        Cell::text(preferred.role),
        Cell::text(preferred.email),
        Cell::text(preferred.state.as_str()),
    )
}
