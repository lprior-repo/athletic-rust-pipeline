use super::{
    cells, compare_jsonl, decode, read, verify_csv_count, verify_jsonl_count, Schema, LIMITS,
};
use crate::bests::SharedSelection;
use crate::export::public_identity::VerifiedPublicIdentityView;
use crate::export::ExportDataset;
use crate::report::ReportResult;
use crate::workbook::publication::verify_sidecars::{cells::Cell, defect};
use crate::workbook::verify::current_school::AffiliationExpectations;
use std::path::Path;

mod resolution;

pub(super) fn verify(
    directory: &Path,
    cohort: &str,
    selections: &[SharedSelection],
    dataset: &ExportDataset,
) -> ReportResult<()> {
    let identities = dataset.verified_public_identities()?;
    let schools = AffiliationExpectations::of(dataset)?;
    verify_csv(
        &directory.join(format!("best-results-{cohort}.csv")),
        selections,
        identities,
        &schools,
    )?;
    verify_jsonl(
        &directory.join(format!("best-results-{cohort}.jsonl")),
        selections,
        identities,
        &schools,
    )
}

fn verify_csv(
    path: &Path,
    selections: &[SharedSelection],
    identities: &VerifiedPublicIdentityView,
    schools: &AffiliationExpectations<'_>,
) -> ReportResult<()> {
    let headers: Vec<_> = Schema::ResultGroups
        .headers()?
        .into_iter()
        .map(|name| match name {
            "school" => "school_at_performance",
            "state" => "state_at_performance",
            other => other,
        })
        .chain([
            "canonical_candidate_id",
            "current_school_id",
            "current_school",
            "current_state",
            "current_school_as_of",
        ])
        .collect();
    let mut position = 0usize;
    let total = read::csv(
        path,
        LIMITS,
        "selected public best-results CSV",
        |index, record| {
            if index == 1 {
                return read::headers(path, record, &headers);
            }
            let selection = selections.get(position).ok_or_else(|| {
                defect("selected public best-results CSV has an unexpected row".into())
            })?;
            let (public_id, candidate_id) = identity(identities, selection)?;
            let mut expected = cells(path, selection, Schema::ResultGroups)?;
            let first = expected
                .first_mut()
                .ok_or_else(|| defect("best-results identity cell is absent".into()))?;
            *first = Cell::text(public_id);
            expected.push(Cell::text(candidate_id));
            expected.extend(
                schools
                    .values(selection.athlete_id().as_str())?
                    .map(Cell::text),
            );
            read::compare_record(path, index, record, &expected)?;
            position = position
                .checked_add(1)
                .ok_or_else(|| defect("selected row counter overflow".into()))?;
            Ok(())
        },
    )?;
    verify_csv_count(path, total, selections.len())
}

fn verify_jsonl(
    path: &Path,
    selections: &[SharedSelection],
    identities: &VerifiedPublicIdentityView,
    schools: &AffiliationExpectations<'_>,
) -> ReportResult<()> {
    let mut position = 0usize;
    let total = read::lines(
        path,
        LIMITS,
        "selected public best-results JSONL",
        |line, text| {
            let selection = selections.get(position).ok_or_else(|| {
                defect("selected public best-results JSONL has an unexpected row".into())
            })?;
            let (public_id, candidate_id) = identity(identities, selection)?;
            let resolution = identities
                .candidate_resolution(selection.athlete_id().as_str())
                .ok_or_else(|| defect("selected public identity resolution is absent".into()))?;
            let selection_wire =
                serde_json::to_value(selection).map_err(|source| decode(path, line, source))?;
            let resolution_wire = resolution::expected(resolution);
            let [school_id, school, state, as_of] =
                schools.values(selection.athlete_id().as_str())?;
            let expected = serde_json::json!({
                "schema": "selected_identity_v2", "athlete_id": public_id,
                "canonical_candidate_id": candidate_id, "identity_resolution": resolution_wire,
                "selection": selection_wire,
                "current_school_id": school_id, "current_school": school,
                "current_state": state, "current_school_as_of": as_of,
            });
            let found = serde_json::from_str(text).map_err(|source| decode(path, line, source))?;
            compare_jsonl(path, line, &expected, &found)?;
            position = position
                .checked_add(1)
                .ok_or_else(|| defect("selected row counter overflow".into()))?;
            Ok(())
        },
    )?;
    verify_jsonl_count(path, total, selections.len())
}

fn identity<'a>(
    identities: &'a VerifiedPublicIdentityView,
    selection: &SharedSelection,
) -> ReportResult<(&'a str, &'a str)> {
    let group = identities
        .group(selection.athlete_id().as_str())
        .ok_or_else(|| {
            defect(
                "selected best-results candidate is absent from retained identity authority".into(),
            )
        })?;
    let public_id = group.public_id().ok_or_else(|| {
        defect("selected best-results candidate is withheld by retained identity authority".into())
    })?;
    Ok((public_id.as_str(), group.canonical_candidate().as_str()))
}

#[cfg(test)]
mod tests;
