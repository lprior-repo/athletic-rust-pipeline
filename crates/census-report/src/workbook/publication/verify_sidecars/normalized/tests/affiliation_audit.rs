use super::TestResult;
use crate::workbook::publication::current_affiliations::fixture;
use crate::workbook::publication::verify_sidecars::input::Inputs;
use crate::workbook::publication::{inventory, normalized};

const FILE: &str = "current_affiliations.csv";

#[test]
fn zero_limit_selected_inventory_keeps_audits_for_excluded_affiliations() -> TestResult {
    let fixture = fixture::fixture()?;
    let inputs = Inputs::for_revision(&fixture.dataset, &fixture.options, 11)?;
    let outcome = inputs.outcome()?;
    check!(outcome.selections.is_empty());
    check!(eq; inputs.derivation.athletes().len(), 1);
    check!(eq; inputs.derivation.affiliation_exclusions().len(), 6);
    let limited = crate::bests::build_selected_outcome(
        &inputs.derivation,
        &inputs.admission,
        fixture.dataset.verified_participation()?,
        Some(0),
    )?;
    check!(limited.selections.is_empty());
    check!(eq; limited.participation_exclusions.len(), outcome.participation_exclusions.len());
    let directory = tempfile::tempdir()?;
    let written = normalized::write_outcome(
        directory.path(),
        &inputs.derivation,
        &fixture.options,
        &outcome,
    )?;
    check!(eq; written.len(), 11);
    check!(eq; written.get(FILE), Some(&7));
    check!(eq; written.get("athletes.csv"), Some(&1));
    let read = super::super::verify_outcome(directory.path(), &inputs, 11, &inputs.outcome()?)?;
    inventory::compare(11, Some(&written), &read)?;
    Ok(())
}

#[test]
fn independent_selected_readback_requires_the_affiliation_audit_family() -> TestResult {
    let fixture = fixture::fixture()?;
    let inputs = Inputs::for_revision(&fixture.dataset, &fixture.options, 11)?;
    let directory = tempfile::tempdir()?;
    normalized::write_outcome(
        directory.path(),
        &inputs.derivation,
        &fixture.options,
        &inputs.outcome()?,
    )?;
    std::fs::remove_file(directory.path().join(FILE))?;
    check!(
        super::super::verify_outcome(directory.path(), &inputs, 11, &inputs.outcome()?,).is_err()
    );
    Ok(())
}
