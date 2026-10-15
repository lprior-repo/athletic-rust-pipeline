use std::path::Path;

use crate::report::ReportResult;

use super::input::Inputs;

mod athletes;
mod best_marks;
mod classes;
mod combined_exclusions;
mod contacts;
mod coverage;
mod facts;
mod inventory;
mod revision;
mod rows;
mod schema;
mod schools;
#[cfg(test)]
mod tests;

#[cfg(test)]
pub(super) fn verify(
    directory: &Path,
    inputs: &Inputs<'_>,
    artifact_revision: u32,
) -> ReportResult<()> {
    let outcome = crate::bests::build_admitted_outcome(
        &inputs.admission,
        &crate::bests::Options {
            scope: inputs.options.scope,
            grad_year: inputs.options.grad_year,
            limit: inputs.options.limit,
            jurisdictions: inputs.options.jurisdictions.clone(),
        },
    )?;
    verify_outcome(directory, inputs, artifact_revision, &outcome).map(|_| ())
}

pub(super) fn verify_outcome(
    directory: &Path,
    inputs: &Inputs<'_>,
    artifact_revision: u32,
    outcome: &crate::bests::SelectionOutcome,
) -> ReportResult<super::super::inventory::RowCounts> {
    super::super::versions::validate_policy(
        artifact_revision,
        inputs.dataset.lineage.policy_revision,
    )?;
    let revision = revision::Revision::parse(artifact_revision)?;
    inventory::verify(inputs)?;
    inputs.admission.ensure_dataset(inputs.dataset)?;
    let bests = &outcome.selections;
    let schools = schools::inventory(inputs, bests)?;
    let coaches = crate::workbook::recruiting::bounded::bounded_claims(
        inputs.derivation.coach_observations(),
        crate::workbook::recruiting::bounded::WORKBOOK_GROUP_BUDGET,
    )?;
    let facts = facts::inventory(inputs, &schools, &coaches, bests, revision)?;
    let mut counts = std::collections::BTreeMap::from([
        (
            "athletes.csv".into(),
            athletes::verify(directory, inputs, bests)?,
        ),
        (
            "schools.csv".into(),
            schools::verify(directory, inputs, &schools)?,
        ),
        (
            "comparison_classes.csv".into(),
            classes::verify(directory, inputs, bests)?,
        ),
        (
            "best_marks.csv".into(),
            best_marks::verify(directory, inputs, bests, revision)?,
        ),
        (
            "coach_contacts.csv".into(),
            contacts::verify(directory, inputs, &coaches, &schools)?,
        ),
        (
            "provenance.csv".into(),
            facts::verify(directory, inputs, &facts)?,
        ),
        ("coverage.csv".into(), coverage::verify(directory, inputs)?),
        (
            "identity_aliases.csv".into(),
            athletes::aliases(directory, inputs)?,
        ),
    ]);
    if revision.has_exclusions() {
        counts.insert(
            "combined_exclusions.csv".into(),
            combined_exclusions::verify(directory, inputs, &outcome.combined_exclusions)?,
        );
    }
    Ok(counts)
}

pub(in crate::workbook) use rows::Row as NormalizedRow;

pub(in crate::workbook) fn visit_athletes(
    path: &Path,
    expected: usize,
    visit: impl FnMut(&NormalizedRow<'_>) -> ReportResult<()>,
) -> ReportResult<u64> {
    rows::read_table(path, schema::ATHLETES, expected, visit)
}
