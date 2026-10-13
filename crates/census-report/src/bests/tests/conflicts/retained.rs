use super::*;

mod classes;
mod store;

use store::reloaded_dataset;

struct NumericCase {
    kind: EventKind,
    conflicting: [Mark; 2],
    fallback: Mark,
    expected: i64,
}

fn numeric_cases() -> TestResult<[NumericCase; 3]> {
    Ok([
        NumericCase {
            kind: EventKind::Track400m,
            conflicting: [
                Mark::TimeSeconds(ExactSeconds::parse("50.00")?),
                Mark::TimeSeconds(ExactSeconds::parse("51.00")?),
            ],
            fallback: Mark::TimeSeconds(ExactSeconds::parse("52.00")?),
            expected: 52_000_000_000,
        },
        NumericCase {
            kind: EventKind::LongJump,
            conflicting: [
                Mark::DistanceMetres(CentiMetres::new(800)),
                Mark::DistanceMetres(CentiMetres::new(750)),
            ],
            fallback: Mark::DistanceMetres(CentiMetres::new(700)),
            expected: 7_000_000,
        },
        NumericCase {
            kind: EventKind::Decathlon,
            conflicting: [
                Mark::Points(CentiPoints::new(500000)),
                Mark::Points(CentiPoints::new(499999)),
            ],
            fallback: Mark::Points(CentiPoints::new(499000)),
            expected: 499000,
        },
    ])
}

fn conflicting_pair(dataset: &ExportDataset, case: &NumericCase) -> [CanonicalPerformance; 2] {
    let first = reported(
        dataset,
        "retained_scalar_conflict",
        case.conflicting[0].clone(),
    );
    let mut revision = first.clone();
    revision.mark = case.conflicting[1].clone();
    [first, revision]
}

fn append_orders(pair: &[CanonicalPerformance; 2]) -> [[CanonicalPerformance; 2]; 2] {
    [
        [pair[0].clone(), pair[1].clone()],
        [pair[1].clone(), pair[0].clone()],
    ]
}

fn retained_conflict(dataset: &ExportDataset, id: &PerformanceId) -> TestResult {
    let performance = dataset
        .performances
        .iter()
        .find(|performance| &performance.id == id)
        .ok_or("missing merged performance")?;
    check!(eq; performance.retained_conflicts.len(), 1);
    check!(eq; performance.retained_conflicts[0].family, "Individual result scalar");
    Ok(())
}

#[test]
fn stored_same_id_numeric_conflict_has_no_best_and_preserves_observations() -> TestResult {
    numeric_cases()?
        .into_iter()
        .try_for_each(|case| -> TestResult {
            let seed = selection_dataset(case.kind.clone())?;
            let pair = conflicting_pair(&seed, &case);
            append_orders(&pair)
                .into_iter()
                .try_for_each(|order| -> TestResult {
                    let dataset = reloaded_dataset(&seed, &order)?;
                    check!(eq; dataset.performances.len(), 1);
                    retained_conflict(&dataset, &pair[0].id)?;
                    check!(eq; selected(&dataset), Vec::new());
                    Ok(())
                })
        })
}

#[test]
fn stored_same_id_numeric_conflict_uses_clean_context_fallback() -> TestResult {
    numeric_cases()?
        .into_iter()
        .try_for_each(|case| -> TestResult {
            let seed = selection_dataset(case.kind.clone())?;
            let pair = conflicting_pair(&seed, &case);
            let mut clean = reported(&seed, "clean_context_fallback", case.fallback.clone());
            clean.date = "2025-02-28".into();
            append_orders(&pair)
                .into_iter()
                .try_for_each(|order| -> TestResult {
                    let observations = [order[0].clone(), order[1].clone(), clean.clone()];
                    let mut dataset = reloaded_dataset(&seed, &observations)?;
                    retained_conflict(&dataset, &pair[0].id)?;
                    let rows = selected(&dataset);
                    dataset.performances.reverse();
                    check!(eq; selected(&dataset), rows);
                    check!(eq; rows.len(), 1);
                    check!(eq; rows[0].source.performance_id, clean.id);
                    check!(eq; rows[0].result.value, case.expected);
                    check!(eq; rows[0].population.marks, 2);
                    check!(eq; rows[0].conflicts.len(), 1);
                    Ok(())
                })
        })
}

#[test]
fn clean_duplicate_cannot_revive_retained_conflict_context() -> TestResult {
    numeric_cases()?
        .into_iter()
        .try_for_each(|case| -> TestResult {
            let seed = selection_dataset(case.kind.clone())?;
            let pair = conflicting_pair(&seed, &case);
            append_orders(&pair)
                .into_iter()
                .try_for_each(|order| -> TestResult {
                    let mut duplicate = order[0].clone();
                    duplicate.id = Id::mint("perf", &["clean_duplicate_of_contested_result"]);
                    duplicate.source_key = "clean_duplicate_of_contested_result".into();
                    duplicate.source_athlete =
                        Some(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "2001"));
                    let observations = [order[0].clone(), order[1].clone(), duplicate];
                    let mut dataset = reloaded_dataset(&seed, &observations)?;
                    check!(eq; dataset.performances.len(), 2);
                    retained_conflict(&dataset, &pair[0].id)?;
                    check!(eq; selected(&dataset), Vec::new());
                    dataset.performances.reverse();
                    check!(eq; selected(&dataset), Vec::new());
                    Ok(())
                })
        })
}

#[test]
fn every_stored_context_contested_has_no_published_best() -> TestResult {
    numeric_cases()?
        .into_iter()
        .try_for_each(|case| -> TestResult {
            let seed = selection_dataset(case.kind.clone())?;
            let pair = conflicting_pair(&seed, &case);
            let mut other = pair[0].clone();
            other.id = Id::mint("perf", &["other_contested_context"]);
            other.source_key = "other_contested_context".into();
            other.date = "2025-02-28".into();
            let mut other_revision = other.clone();
            other_revision.mark = pair[1].mark.clone();
            append_orders(&pair)
                .into_iter()
                .try_for_each(|order| -> TestResult {
                    let mut observations = vec![
                        order[0].clone(),
                        order[1].clone(),
                        other.clone(),
                        other_revision.clone(),
                    ];
                    let mut dataset = reloaded_dataset(&seed, &observations)?;
                    retained_conflict(&dataset, &pair[0].id)?;
                    retained_conflict(&dataset, &other.id)?;
                    check!(eq; selected(&dataset), Vec::new());
                    dataset.performances.reverse();
                    check!(eq; selected(&dataset), Vec::new());
                    observations.reverse();
                    check!(eq; selected(&reloaded_dataset(&seed, &observations)?), Vec::new());
                    Ok(())
                })
        })
}
