use super::fixture::{numeric_cases, orders, NumericCase, Observation, TieFixture};
use super::selection::{assert_winner, provider_evidence, same_context, stable_id_reports};
use crate::bests::tests::{selected, TestResult};
use crate::bests::SharedSelection;
use crate::export::ExportDataset;
use census_domain::model::{ExactSeconds, Mark, PerformanceId};

fn reports(fixture: &TieFixture, case: &NumericCase) -> TestResult<[Observation; 3]> {
    let [preferred, other] = stable_id_reports(fixture, case)?;
    let mut preferred = preferred.with_mark(case.better.clone())?;
    let mut other = same_context(other.with_mark(case.better.clone())?, &preferred);
    provider_evidence(&mut preferred, "wiaa_results")?;
    provider_evidence(&mut other, "milesplit")?;
    let later = fixture
        .report(case, "support-later", "2026-04-02")?
        .with_mark(case.better.clone())?;
    check!(preferred.performance.id != other.performance.id);
    check!(preferred.performance.source_key != other.performance.source_key);
    Ok([preferred, other, later])
}

fn expected_ids(observations: &[Observation]) -> Vec<PerformanceId> {
    let mut ids: Vec<_> = observations
        .iter()
        .map(|observation| observation.performance.id.clone())
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

fn assert_support(row: &SharedSelection, observations: &[Observation]) -> TestResult {
    check!(eq; row.supporting_tied_candidate_ids(), expected_ids(observations).as_slice());
    Ok(())
}

#[test]
fn tied_support_retains_distinct_same_context_source_ids_without_inflating_counts() -> TestResult {
    let fixture = TieFixture::new()?;
    numeric_cases()?.iter().try_for_each(|case| -> TestResult {
        let observations = reports(&fixture, case)?;
        let mut with_inferior = observations.to_vec();
        with_inferior.push(fixture.report(case, "support-inferior", "2024-02-29")?);
        let directory = tempfile::tempdir()?;
        let store = fixture.store_with_replay(directory.path(), &with_inferior)?;
        let row = assert_winner(
            &ExportDataset::load(&store)?,
            observations.first().ok_or("missing preferred support")?,
        )?;
        check!(eq; row.eligible_candidate_count, 3);
        check!(eq; row.tied_performance_count, 2);
        check!(eq; row.tied_performance_ids.len(), 2);
        assert_support(&row, &observations)
    })
}

#[test]
fn tied_support_is_arrival_order_and_replay_invariant() -> TestResult {
    let fixture = TieFixture::new()?;
    numeric_cases()?.iter().try_for_each(|case| -> TestResult {
        let observations = reports(&fixture, case)?;
        let preferred = observations.first().ok_or("missing preferred support")?;
        orders(&observations)?
            .into_iter()
            .try_for_each(|mut order| -> TestResult {
                order.push(preferred.clone());
                let directory = tempfile::tempdir()?;
                let store = fixture.store_with_replay(directory.path(), &order)?;
                let mut dataset = ExportDataset::load(&store)?;
                let repeated = dataset
                    .performances
                    .first()
                    .cloned()
                    .ok_or("missing replay candidate")?;
                dataset.performances.push(repeated);
                let row = assert_winner(&dataset, preferred)?;
                check!(eq; row.eligible_candidate_count, 2);
                check!(eq; row.tied_performance_count, 2);
                assert_support(&row, &observations)?;
                dataset.performances.reverse();
                check!(eq; assert_winner(&dataset, preferred)?, row);
                Ok(())
            })
    })
}

#[test]
fn tied_support_keeps_equal_precision_source_spellings() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let case = cases.first().ok_or("missing time fixture")?;
    let [preferred, other] = stable_id_reports(&fixture, case)?;
    let other = same_context(
        other.with_mark(Mark::TimeSeconds(ExactSeconds::parse("50.0")?))?,
        &preferred,
    );
    let later = fixture
        .report(case, "support-precision-later", "2026-04-02")?
        .with_mark(Mark::TimeSeconds(ExactSeconds::parse("50.000")?))?;
    let observations = [preferred.clone(), other, later];
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &observations)?;
    let row = assert_winner(&ExportDataset::load(&store)?, &preferred)?;
    check!(eq; row.tied_performance_count, 2);
    assert_support(&row, &observations)
}

#[test]
fn tied_support_excludes_conflicted_and_withdrawn_contexts() -> TestResult {
    let fixture = TieFixture::new()?;
    numeric_cases()?.iter().try_for_each(|case| -> TestResult {
        [case.better.clone(), Mark::Raw("SCR".into())]
            .into_iter()
            .try_for_each(|revision| -> TestResult {
                let [contested, other] = stable_id_reports(&fixture, case)?;
                let other = same_context(other, &contested);
                let revision = contested.clone().with_mark(revision)?;
                let clean = fixture.report(case, "support-clean", "2026-04-02")?;
                let observations = [contested, revision, other, clean.clone()];
                for order in [
                    observations.to_vec(),
                    observations.iter().rev().cloned().collect(),
                ] {
                    let directory = tempfile::tempdir()?;
                    let store = fixture.store_with_replay(directory.path(), &order)?;
                    let row = assert_winner(&ExportDataset::load(&store)?, &clean)?;
                    check!(eq; row.eligible_candidate_count, 1);
                    check!(eq; row.tied_performance_count, 1);
                    assert_support(&row, std::slice::from_ref(&clean))?;
                }
                Ok(())
            })
    })
}

#[test]
fn tied_support_is_internal_and_does_not_change_serialized_selection() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let observations = reports(&fixture, cases.first().ok_or("missing time fixture")?)?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &observations)?;
    let mut row = selected(&ExportDataset::load(&store)?)?
        .into_iter()
        .next()
        .ok_or("missing supported selection")?;
    assert_support(&row, &observations)?;
    let encoded = serde_json::to_value(&row)?;
    check!(encoded.get("supporting_tied_candidate_ids").is_none());
    row.supporting_tied_candidate_ids.clear();
    check!(eq; serde_json::to_value(&row)?, encoded);
    Ok(())
}
