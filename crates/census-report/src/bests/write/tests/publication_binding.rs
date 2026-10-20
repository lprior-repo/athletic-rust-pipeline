use super::*;
use crate::export::ExportDataset;
use crate::report::ReportError;
use census_domain::model::{ContractRefV1, ProfileDate, ProfileId};
use census_store::Store;

#[test]
fn naked_bests_writer_refuses_empty_selected_output_before_creating_files() -> TestResult {
    for policy in [19, 20, 21] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path().join("store"))?;
        let mut dataset = ExportDataset::load(&store)?;
        dataset.bind_historical_policy_for_test(policy)?;
        dataset.bind_selection_for_test(
            ProfileId::parse(&"a".repeat(64))?,
            ProfileDate::parse("2026-10-01")?,
        )?;
        let output = directory.path().join("not-created");
        refuse_selected(crate::bests::write(&output, &[], "co2027", &dataset))?;
        check!(!output.exists());
        dataset.bind_participation_rule_for_test(ContractRefV1::new(1, 1, &"b".repeat(64))?)?;
        refuse_selected(crate::bests::write(&output, &[], "co2027", &dataset))?;
        check!(!output.exists());
    }
    Ok(())
}

#[test]
fn naked_bests_writer_does_not_replace_existing_files_from_selected_rows() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let mut dataset = ExportDataset::load(&store)?;
    let rows = vec![selection(
        ORDINARY_NAME,
        "Synthetic School",
        "Synthetic Meet",
        Mark::Raw("11.00".into()),
        None,
    )];
    let output = directory.path().join("bests");
    let (jsonl, csv) = crate::bests::write(&output, &rows, "co2027", &dataset)?;
    let json_before = std::fs::read(&jsonl)?;
    let csv_before = std::fs::read(&csv)?;
    dataset.bind_selection_for_test(
        ProfileId::parse(&"a".repeat(64))?,
        ProfileDate::parse("2026-10-01")?,
    )?;
    dataset.bind_participation_rule_for_test(ContractRefV1::new(1, 1, &"b".repeat(64))?)?;
    for rows in [rows.as_slice(), &[]] {
        refuse_selected(crate::bests::write(&output, rows, "co2027", &dataset))?;
        check!(eq; std::fs::read(&jsonl)?, json_before);
        check!(eq; std::fs::read(&csv)?, csv_before);
    }
    Ok(())
}

#[test]
fn historical_bests_writer_uses_its_dataset_policy_for_empty_output() -> TestResult {
    for (policy, columns) in [(19, 36), (20, 36), (21, 39)] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path().join("store"))?;
        let mut dataset = ExportDataset::load(&store)?;
        dataset.bind_historical_policy_for_test(policy)?;
        let (jsonl, csv) = crate::bests::write(directory.path(), &[], "empty", &dataset)?;
        check!(eq; std::fs::read_to_string(jsonl)?, "");
        let mut reader = csv::Reader::from_path(csv)?;
        check!(eq; reader.headers()?.len(), columns);
        check!(eq; reader.records().count(), 0);
    }
    Ok(())
}

fn refuse_selected<T>(result: crate::report::ReportResult<T>) -> TestResult {
    match result {
        Err(ReportError::Invariant { detail }) => {
            check!(eq; detail, "selected participation policy requires verified frozen evidence");
            Ok(())
        }
        Err(error) => Err(format!("unexpected selected bests refusal: {error}").into()),
        Ok(_) => Err("naked bests writer accepted a selected dataset".into()),
    }
}
