use super::*;
use crate::export::{ExportDataset, VerifiedCurrentAffiliations, VerifiedParticipationView};
use crate::report::{Derivation, ReportError, Scope};
use census_domain::model::{ContractRefV1, ProfileDate, ProfileId};
use std::collections::BTreeMap;

#[test]
fn verified_selected_dataset_does_not_authorize_naked_rows_or_empty_output() -> TestResult {
    let directory = tempfile::tempdir()?;
    let dataset = selected_dataset(directory.path())?;
    let rows = [selection(
        ORDINARY_NAME,
        "Unrelated Synthetic School",
        "Unrelated Synthetic Meet",
        Mark::Raw("11.00".into()),
        None,
    )];
    dataset.verified_participation()?;
    dataset.verified_current_affiliations()?;
    for rows in [rows.as_slice(), &[]] {
        let output = directory.path().join("uncreated");
        refusal(crate::bests::write(&output, rows, "selected", &dataset))?;
        check!(!output.exists());
    }
    Ok(())
}

#[test]
fn verified_selected_dataset_cannot_replace_prior_naked_writer_artifacts() -> TestResult {
    let directory = tempfile::tempdir()?;
    let dataset = selected_dataset(directory.path())?;
    let output = directory.path().join("existing");
    std::fs::create_dir(&output)?;
    let jsonl = output.join("best-results-selected.jsonl");
    let csv = output.join("best-results-selected.csv");
    std::fs::write(&jsonl, b"prior jsonl\n")?;
    std::fs::write(&csv, b"prior csv\n")?;
    refusal(crate::bests::write(&output, &[], "selected", &dataset))?;
    check!(eq; std::fs::read(jsonl)?.as_slice(), b"prior jsonl\n");
    check!(eq; std::fs::read(csv)?.as_slice(), b"prior csv\n");
    Ok(())
}

#[test]
fn public_derivation_requires_identity_authority_before_outcome_writer_effects() -> TestResult {
    let directory = tempfile::tempdir()?;
    let dataset = selected_dataset(directory.path())?;
    let output = directory.path().join("uncreated-outcome");
    match Derivation::of(&dataset, Scope::AllSources, None, &[]) {
        Err(ReportError::Invariant { detail }) => {
            check!(eq; detail, "selected publication requires retained public-identity verification");
        }
        Err(error) => return Err(format!("unexpected derivation refusal: {error}").into()),
        Ok(_) => return Err("selected derivation accepted without identity authority".into()),
    }
    check!(!output.exists());
    Ok(())
}

fn selected_dataset(root: &std::path::Path) -> TestResult<ExportDataset> {
    let store = census_store::Store::open(root.join("store"))?;
    let mut dataset = ExportDataset::load(&store)?;
    let profile = ProfileId::parse(&"a".repeat(64))?;
    let cutoff = ProfileDate::parse("2026-10-01")?;
    let rule = ContractRefV1::new(1, 1, &"b".repeat(64))?;
    dataset.bind_selection_for_test(profile.clone(), cutoff.clone())?;
    dataset.bind_participation_rule_for_test(rule.clone())?;
    let generation = dataset.lineage.input_generation.clone();
    dataset.bind_verified_views_for_test(
        VerifiedParticipationView::fixture(
            generation.clone(),
            profile.clone(),
            rule,
            BTreeMap::new(),
        ),
        VerifiedCurrentAffiliations::fixture(generation, profile, cutoff, BTreeMap::new()),
    )?;
    Ok(dataset)
}

fn refusal<T>(result: crate::report::ReportResult<T>) -> TestResult {
    match result {
        Err(ReportError::Invariant { detail }) => {
            check!(eq; detail, "selected participation policy requires verified frozen evidence");
            Ok(())
        }
        Err(error) => Err(format!("unexpected naked writer refusal: {error}").into()),
        Ok(_) => Err("verified selected input authorized naked selected rows".into()),
    }
}
