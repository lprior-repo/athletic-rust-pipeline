mod identity;
mod provenance;
mod terminal;

use super::fixture::{numeric_cases, NumericCase, Observation, TieFixture};
use super::selection::{provider_evidence, same_context};
use crate::bests::tests::{selected, TestResult};
use crate::bests::SharedSelection;
use crate::export::ExportDataset;
use census_domain::model::{CombinedComponentResult, CombinedEventCompletion, RetainedConflict};

fn reports(fixture: &TieFixture, case: &NumericCase) -> TestResult<[Observation; 2]> {
    let mut first = fixture.report(case, "result-group-one", "2026-04-01")?;
    first.performance.round = None;
    first.performance.heat = None;
    let second = fixture.report(case, "result-group-two", "2026-04-01")?;
    let mut second = same_context(second, &first);
    second.performance.round = None;
    second.performance.heat = None;
    check!(first.performance.id != second.performance.id);
    Ok([first, second])
}

fn rows(dataset: &mut ExportDataset, policy: u32) -> TestResult<Vec<SharedSelection>> {
    let original_policy = dataset.lineage.policy_revision;
    dataset.lineage.policy_revision = policy;
    let result = selected(dataset);
    dataset.lineage.policy_revision = original_policy;
    Ok(result?)
}

fn single(dataset: &mut ExportDataset, policy: u32) -> TestResult<SharedSelection> {
    let mut selected = rows(dataset, policy)?;
    check!(eq; selected.len(), 1);
    selected
        .pop()
        .ok_or_else(|| "missing selected result".into())
}

#[test]
fn distinct_equal_results_with_missing_context_count_as_two_result_groups() -> TestResult {
    let fixture = TieFixture::new()?;
    for case in numeric_cases()? {
        let observations = reports(&fixture, &case)?;
        let directory = tempfile::tempdir()?;
        let store = fixture.store_with_replay(directory.path(), &observations)?;
        let mut dataset = ExportDataset::load(&store)?;
        let current = single(&mut dataset, 21)?;
        check!(eq; current.inventory.eligible_group_count(), 2);
        check!(eq; current.inventory.tied_group_count(), 2);
        check!(eq; current.supporting_tied_candidate_ids().len(), 2);
        let historical = single(&mut dataset, 20)?;
        check!(eq; historical.inventory.eligible_group_count(), 1);
        check!(eq; historical.inventory.tied_group_count(), 1);
        check!(eq; historical.supporting_tied_candidate_ids().len(), 2);
    }
    Ok(())
}

#[test]
fn distinct_unequal_results_do_not_manufacture_a_conflict() -> TestResult {
    let fixture = TieFixture::new()?;
    for case in numeric_cases()? {
        let [first, second] = reports(&fixture, &case)?;
        let first = first.with_mark(case.better.clone())?;
        let directory = tempfile::tempdir()?;
        let store = fixture.store_with_replay(directory.path(), &[first.clone(), second])?;
        let mut dataset = ExportDataset::load(&store)?;
        let current = single(&mut dataset, 21)?;
        check!(eq; current.source.performance_id, first.performance.id);
        check!(eq; current.inventory.eligible_group_count(), 2);
        check!(eq; current.inventory.tied_group_count(), 1);
        check!(current.conflicts.is_empty());
        check!(rows(&mut dataset, 20)?.is_empty());
    }
    Ok(())
}

#[test]
fn retained_conflict_excludes_its_result_group_without_poisoning_the_context() -> TestResult {
    let fixture = TieFixture::new()?;
    for case in numeric_cases()? {
        let [mut first, second] = reports(&fixture, &case)?;
        first
            .performance
            .retained_conflicts
            .push(RetainedConflict::new(
                "mark",
                first.performance.id.as_str(),
                &first.performance.source_key,
                "incompatible retained correction",
            ));
        let directory = tempfile::tempdir()?;
        let store = fixture.store_with_replay(directory.path(), &[first, second.clone()])?;
        let mut dataset = ExportDataset::load(&store)?;
        let current = single(&mut dataset, 21)?;
        check!(eq; current.source.performance_id, second.performance.id);
        check!(eq; current.inventory.eligible_group_count(), 1);
        check!(eq; current.inventory.tied_group_count(), 1);
        check!(rows(&mut dataset, 20)?.is_empty());
    }
    Ok(())
}

#[test]
fn exact_id_replay_does_not_inflate_result_group_inventory() -> TestResult {
    let fixture = TieFixture::new()?;
    for case in numeric_cases()? {
        let observations = reports(&fixture, &case)?;
        let directory = tempfile::tempdir()?;
        let store = fixture.store_with_replay(directory.path(), &observations)?;
        let mut dataset = ExportDataset::load(&store)?;
        let expected = single(&mut dataset, 21)?;
        dataset.performances.push(
            observations
                .first()
                .ok_or("replayed result")?
                .performance
                .clone(),
        );
        let replayed = single(&mut dataset, 21)?;
        check!(eq; replayed.inventory, expected.inventory);
        check!(eq; replayed.supporting_tied_candidate_ids(), expected.supporting_tied_candidate_ids());
        check!(eq; replayed.source, expected.source);
        dataset.performances.reverse();
        check!(eq; single(&mut dataset, 21)?, replayed);
    }
    Ok(())
}

#[test]
fn contradictory_same_id_reports_exclude_only_that_result_group() -> TestResult {
    let fixture = TieFixture::new()?;
    for case in numeric_cases()? {
        let [first, second] = reports(&fixture, &case)?;
        let directory = tempfile::tempdir()?;
        let store =
            fixture.store_with_replay(directory.path(), &[first.clone(), second.clone()])?;
        let mut dataset = ExportDataset::load(&store)?;
        dataset
            .performances
            .push(first.with_mark(case.better)?.performance);
        let current = single(&mut dataset, 21)?;
        check!(eq; current.source.performance_id, second.performance.id);
        check!(eq; current.inventory.eligible_group_count(), 1);
        check!(eq; current.inventory.tied_group_count(), 1);
        check!(eq; current.conflicts.len(), 1);
        check!(rows(&mut dataset, 20)?.is_empty());
    }
    Ok(())
}

#[test]
fn different_provider_reports_do_not_claim_known_physical_cardinality() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let case = cases.first().ok_or("time case")?;
    let [mut first, mut second] = reports(&fixture, case)?;
    provider_evidence(&mut first, "milesplit")?;
    provider_evidence(&mut second, "athletic_net")?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[first, second])?;
    let mut dataset = ExportDataset::load(&store)?;
    let current = single(&mut dataset, 21)?;
    let encoded = serde_json::to_value(current)?;
    check!(eq; encoded.get("eligible_result_group_count"), Some(&serde_json::json!(2)));
    check!(eq; encoded.get("tied_result_group_count"), Some(&serde_json::json!(2)));
    check!(eq; encoded.get("result_group_basis"), Some(&serde_json::json!("canonical_performance_id")));
    check!(eq; encoded.get("physical_performance_count_status"), Some(&serde_json::json!("unknown")));
    check!(eq; encoded.get("selection_policy_version"), Some(&serde_json::json!(21)));
    for field in [
        "eligible_candidate_count",
        "tied_performance_count",
        "tied_performance_ids",
    ] {
        check!(encoded.get(field).is_none(), "legacy count key {field}");
    }
    Ok(())
}

#[test]
fn historical_inventory_serialization_retains_its_original_keys() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let observations = reports(&fixture, cases.first().ok_or("time case")?)?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &observations)?;
    let mut dataset = ExportDataset::load(&store)?;
    for policy in [18, 19, 20] {
        let encoded = serde_json::to_value(single(&mut dataset, policy)?)?;
        check!(eq; encoded.get("eligible_candidate_count"), Some(&serde_json::json!(1)));
        check!(eq; encoded.get("tied_performance_count"), Some(&serde_json::json!(1)));
        check!(encoded.get("tied_performance_ids").is_some());
        for field in [
            "eligible_result_group_count",
            "tied_result_group_count",
            "tied_result_group_representative_ids",
            "result_group_basis",
            "physical_performance_count_status",
            "selection_policy_version",
        ] {
            check!(encoded.get(field).is_none(), "new count key {field}");
        }
    }
    Ok(())
}

#[test]
fn terminal_combined_result_excludes_only_its_canonical_result_id() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let case = cases.last().ok_or("combined case")?;
    for status in [
        CombinedComponentResult::DidNotFinish,
        CombinedComponentResult::DidNotStart,
        CombinedComponentResult::Disqualified,
    ] {
        let [mut terminal, complete] = reports(&fixture, case)?;
        let completion = terminal
            .performance
            .combined_completion
            .as_ref()
            .ok_or("completion")?;
        let mut components = completion.components().to_vec();
        *components.first_mut().ok_or("first component")? = status;
        terminal.performance.combined_completion = Some(CombinedEventCompletion::new(
            terminal
                .event
                .specification
                .combined_event
                .clone()
                .ok_or("specification")?,
            components,
        )?);
        let directory = tempfile::tempdir()?;
        let store = fixture.store_with_replay(directory.path(), &[terminal, complete.clone()])?;
        let mut dataset = ExportDataset::load(&store)?;
        let current = single(&mut dataset, 21)?;
        check!(eq; current.source.performance_id, complete.performance.id);
        check!(eq; current.inventory.eligible_group_count(), 1);
        check!(rows(&mut dataset, 20)?.is_empty());
    }
    Ok(())
}
