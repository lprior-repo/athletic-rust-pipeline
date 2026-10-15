use super::TestResult;
use crate::workbook::publication::verify_sidecars::input::Inputs;
use crate::workbook::publication::{inventory, normalized, selected::fixture::Fixture};

#[test]
fn revision_seven_writes_and_independently_reads_all_exclusions_at_zero_row_limit() -> TestResult {
    let fixture = Fixture::new()?;
    let inputs = Inputs::for_revision(&fixture.dataset, &fixture.options, 7)?;
    let outcome = inputs.outcome()?;
    check!(outcome.selections.is_empty());
    check!(eq; outcome.participation_exclusions.len(), 1);
    let directory = tempfile::tempdir()?;
    let written = normalized::write_outcome(
        directory.path(),
        &inputs.derivation,
        &fixture.options,
        &outcome,
    )?;
    check!(eq; written.len(), 11);
    check!(eq; written.get("participation_exclusions.csv"), Some(&1));
    let read = super::super::verify_outcome(directory.path(), &inputs, 7, &inputs.outcome()?)?;
    inventory::compare(7, Some(&written), &read)?;
    let text = std::fs::read_to_string(directory.path().join("participation_exclusions.csv"))?;
    check!(text.contains("missing_participation_evidence"));
    check!(text.contains("unresolved"));
    check!(super::super::verify_outcome(directory.path(), &inputs, 6, &outcome).is_err());
    Ok(())
}

#[test]
fn revision_seven_readback_refuses_missing_duplicate_foreign_and_tampered_exclusions() -> TestResult
{
    let fixture = Fixture::new()?;
    let inputs = Inputs::for_revision(&fixture.dataset, &fixture.options, 7)?;
    let directory = tempfile::tempdir()?;
    let outcome = inputs.outcome()?;
    let path = directory.path().join("participation_exclusions.csv");
    for mutation in ["absent", "missing", "duplicate", "foreign", "tampered"] {
        normalized::write_outcome(
            directory.path(),
            &inputs.derivation,
            &fixture.options,
            &outcome,
        )?;
        if mutation == "absent" {
            std::fs::remove_file(&path)?;
        } else {
            let mut reader = csv::Reader::from_path(&path)?;
            let header = reader.headers()?.clone();
            let row = reader
                .records()
                .next()
                .ok_or("missing synthetic exclusion")??;
            drop(reader);
            let mut writer = csv::Writer::from_path(&path)?;
            writer.write_record(&header)?;
            match mutation {
                "missing" => {}
                "duplicate" => {
                    writer.write_record(&row)?;
                    writer.write_record(&row)?;
                }
                "foreign" | "tampered" => {
                    let mut fields: Vec<String> = row.iter().map(str::to_owned).collect();
                    let column = if mutation == "foreign" { 1 } else { 6 };
                    *fields.get_mut(column).ok_or("synthetic cell")? =
                        "foreign-synthetic-value".into();
                    writer.write_record(fields)?;
                }
                _ => return Err("unknown synthetic mutation".into()),
            }
            writer.flush()?;
        }
        check!(
            super::super::verify_outcome(directory.path(), &inputs, 7, &inputs.outcome()?).is_err()
        );
    }
    Ok(())
}

#[test]
fn public_selected_derivation_writes_verified_output_and_refuses_unbound_revisions() -> TestResult {
    let fixture = Fixture::new()?;
    let inputs = Inputs::for_revision(&fixture.dataset, &fixture.options, 7)?;
    let outcome = inputs.outcome()?;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("not-created");
    let selected = crate::report::Derivation::of(
        &fixture.dataset,
        fixture.options.scope,
        fixture.options.grad_year,
        &fixture.options.jurisdictions,
    )?;
    let written = normalized::write_outcome(&path, &selected, &fixture.options, &outcome)?;
    check!(eq; written.get("participation_exclusions.csv"), Some(&1));
    check!(path.join("participation_exclusions.csv").is_file());
    check!(Inputs::for_revision(&fixture.dataset, &fixture.options, 6).is_err());
    let store = census_store::Store::open(directory.path().join("store"))?;
    let dataset = crate::export::ExportDataset::load(&store)?;
    check!(Inputs::for_revision(&dataset, &fixture.options, 7).is_err());
    Ok(())
}
