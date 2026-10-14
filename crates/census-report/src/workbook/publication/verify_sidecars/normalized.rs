use std::path::Path;

use crate::report::ReportResult;

use super::input::Inputs;

mod athletes;
mod best_marks;
mod classes;
mod contacts;
mod coverage;
mod facts;
mod inventory;
mod rows;
mod schema;
mod schools;
#[cfg(test)]
mod tests;

pub(super) fn verify(directory: &Path, inputs: &Inputs<'_>) -> ReportResult<()> {
    inventory::verify(inputs)?;
    inputs.admission.ensure_dataset(inputs.dataset)?;
    let bests = crate::bests::build_from_dataset(
        inputs.dataset,
        &crate::bests::Options {
            scope: inputs.options.scope,
            grad_year: inputs.options.grad_year,
            limit: inputs.options.limit,
            jurisdictions: inputs.options.jurisdictions.clone(),
        },
    )?;
    let schools = schools::inventory(inputs, &bests)?;
    let coaches = crate::workbook::recruiting::bounded::bounded_claims(
        inputs.derivation.coach_observations(),
        crate::workbook::recruiting::bounded::WORKBOOK_GROUP_BUDGET,
    )?;
    let facts = facts::inventory(inputs, &schools, &coaches, &bests)?;
    athletes::verify(directory, inputs, &bests)?;
    schools::verify(directory, inputs, &schools)?;
    classes::verify(directory, inputs, &bests)?;
    best_marks::verify(directory, inputs, &bests)?;
    contacts::verify(directory, inputs, &coaches, &schools)?;
    facts::verify(directory, inputs, &facts)?;
    coverage::verify(directory, inputs)?;
    athletes::aliases(directory, inputs)
}
