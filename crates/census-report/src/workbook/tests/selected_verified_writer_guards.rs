use super::TestResult;
use crate::export::{ExportDataset, VerifiedCurrentAffiliations, VerifiedParticipationView};
use crate::report::{ReportError, Scope};
use crate::workbook::Options;
use census_domain::model::{ContractRefV1, ProfileDate, ProfileId, SchoolYear};
use std::collections::BTreeMap;

#[test]
fn naked_normalized_writer_refuses_even_fully_verified_selected_zero_limit() -> TestResult {
    for limit in [None, Some(0)] {
        let directory = tempfile::tempdir()?;
        let store = census_store::Store::open(directory.path().join("store"))?;
        let dataset = selected_dataset(&store)?;
        dataset.verified_participation()?;
        dataset.verified_current_affiliations()?;
        let output = directory.path().join("uncreated");
        let result = crate::workbook::publication::normalized::write(
            &output,
            &dataset,
            &Options {
                grad_year: None,
                out: None,
                limit,
                scope: Scope::AllSources,
                jurisdictions: Vec::new(),
                school_year: SchoolYear::DEFAULT,
            },
            &[],
            &[],
        );
        match result {
            Err(ReportError::Invariant { detail }) => {
                check!(eq; detail, "selected participation policy requires verified frozen evidence");
            }
            other => return Err(format!("unexpected naked normalized result: {other:?}").into()),
        }
        check!(!output.exists());
    }
    Ok(())
}

fn selected_dataset(store: &census_store::Store) -> TestResult<ExportDataset> {
    let mut dataset = ExportDataset::load(store)?;
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
