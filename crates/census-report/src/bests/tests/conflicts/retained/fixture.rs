use super::*;
use crate::bests::tests::fixture::{completion_with_total, TieFixture};

pub(super) fn numeric_dataset(kind: EventKind) -> TestResult<ExportDataset> {
    let case = crate::bests::tests::fixture::numeric_cases()?
        .into_iter()
        .find(|case| case.kind == kind)
        .ok_or("missing numeric conflict fixture")?;
    let fixture = TieFixture::new()?;
    let observation = fixture.report(&case, "retained_conflict_seed", "2025-03-01")?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[observation])?;
    let mut dataset = ExportDataset::load(&store)?;
    dataset.lineage.policy_revision = 20;
    let rows = selected(&dataset)?;
    check!(eq; rows.len(), 1);
    let winner = rows
        .first()
        .ok_or("uncontested numeric fixture has no best")?;
    check!(eq; winner.result.value, case.value);
    Ok(dataset)
}

pub(super) fn numeric_report(
    dataset: &ExportDataset,
    label: &str,
    mark: Mark,
) -> TestResult<CanonicalPerformance> {
    let mut performance = dataset
        .performances
        .first()
        .cloned()
        .ok_or("missing owned numeric fixture performance")?;
    performance.id = Id::mint("perf", &[label]);
    performance.source_key = label.into();
    with_mark(dataset, performance, mark)
}

pub(super) fn with_mark(
    dataset: &ExportDataset,
    mut performance: CanonicalPerformance,
    mark: Mark,
) -> TestResult<CanonicalPerformance> {
    let event = dataset
        .events
        .iter()
        .find(|event| event.id == performance.event)
        .ok_or("missing numeric fixture event")?;
    performance.combined_completion = match (&event.specification.combined_event, &mark) {
        (Some(specification), Mark::Points(total)) => {
            Some(completion_with_total(specification.clone(), *total)?)
        }
        _ => None,
    };
    performance.mark = mark;
    Ok(performance)
}
