use super::*;

#[test]
fn selection_skips_distance_reports_for_timed_events() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track400m)?;
    let valid = reported(
        &dataset,
        "valid_time",
        Mark::TimeSeconds(ExactSeconds::parse("50.00")?),
    );
    let incompatible = reported(
        &dataset,
        "historical_distance",
        Mark::DistanceMetres(CentiMetres::new(40000)),
    );
    dataset.performances = vec![incompatible, valid.clone()];

    let rows = selected(&dataset);

    check!(eq; rows.len(), 1);
    check!(eq; rows[0].source.performance_id, valid.id);
    check!(eq; rows[0].result.mark, valid.mark);
    check!(eq; rows[0].result.value, 50_000_000_000);
    check!(eq; rows[0].population.marks, 1);
    check!(eq; rows[0].conflicts, Vec::new());
    Ok(())
}

#[test]
fn selection_skips_time_reports_for_combined_events() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Decathlon)?;
    let valid = reported(
        &dataset,
        "valid_points",
        Mark::Points(CentiPoints::new(345600)),
    );
    let incompatible = reported(
        &dataset,
        "historical_time",
        Mark::TimeSeconds(ExactSeconds::parse("34.56")?),
    );
    dataset.performances = vec![incompatible, valid.clone()];

    let rows = selected(&dataset);

    check!(eq; rows.len(), 1);
    check!(eq; rows[0].source.performance_id, valid.id);
    check!(eq; rows[0].result.mark, valid.mark);
    check!(eq; rows[0].result.value, 345600);
    check!(eq; rows[0].population.marks, 1);
    check!(eq; rows[0].conflicts, Vec::new());
    Ok(())
}

#[test]
fn selection_skips_points_reports_for_ordinary_field_events() -> TestResult {
    let mut dataset = selection_dataset(EventKind::ShotPut)?;
    let valid = reported(
        &dataset,
        "valid_distance",
        Mark::DistanceMetres(CentiMetres::new(1200)),
    );
    let incompatible = reported(
        &dataset,
        "historical_points",
        Mark::Points(CentiPoints::new(345600)),
    );
    dataset.performances = vec![incompatible, valid.clone()];

    let rows = selected(&dataset);

    check!(eq; rows.len(), 1);
    check!(eq; rows[0].source.performance_id, valid.id);
    check!(eq; rows[0].result.mark, valid.mark);
    check!(eq; rows[0].result.value, 12_000_000);
    check!(eq; rows[0].population.marks, 1);
    check!(eq; rows[0].conflicts, Vec::new());
    Ok(())
}

#[test]
fn selection_publishes_no_pr_when_only_incompatible_marks_exist() -> TestResult {
    for (kind, mark) in [
        (
            EventKind::Track400m,
            Mark::DistanceMetres(CentiMetres::new(40000)),
        ),
        (
            EventKind::Decathlon,
            Mark::TimeSeconds(ExactSeconds::parse("34.56")?),
        ),
        (EventKind::ShotPut, Mark::Points(CentiPoints::new(345600))),
    ] {
        let mut dataset = selection_dataset(kind)?;
        dataset.performances = vec![reported(&dataset, "historical_incompatible", mark)];

        check!(eq; selected(&dataset), Vec::new());
    }
    Ok(())
}
