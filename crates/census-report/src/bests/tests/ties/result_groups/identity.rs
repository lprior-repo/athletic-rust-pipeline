use super::*;
use census_domain::model::{
    Adjustment, CentiPoints, EventKind, ExactSeconds, GradYear, Id, Mark, SourceRef,
};

#[test]
fn same_id_contradiction_is_checked_before_unknown_or_out_of_cohort_filtering() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let [first, _] = reports(&fixture, cases.first().ok_or("ordinary case")?)?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&first))?;
    let mut dataset = ExportDataset::load(&store)?;
    let unknown = Id::mint("ath", &["unknown-owner"]);
    let mut out_of_cohort = dataset.athletes.first().ok_or("athlete")?.clone();
    out_of_cohort.id = Id::mint("ath", &["other-cohort"]);
    out_of_cohort.grad_year = GradYear::new(2028).ok_or("cohort")?;
    out_of_cohort.published_graduations.clear();
    dataset.athletes.push(out_of_cohort.clone());
    let mut admitted = dataset.athletes.first().ok_or("athlete")?.clone();
    admitted.id = Id::mint("ath", &["different-admitted-owner"]);
    dataset.athletes.push(admitted.clone());
    for owner in [unknown, out_of_cohort.id, admitted.id] {
        let mut other = first.performance.clone();
        other.athlete = owner;
        dataset.performances = vec![first.performance.clone(), other];
        for _ in 0..2 {
            let error = rows(&mut dataset, 21)
                .err()
                .ok_or("filtered contradiction accepted")?;
            check!(error
                .to_string()
                .contains("incompatible retained identity or context"));
            dataset.performances.reverse();
        }
    }
    Ok(())
}

#[test]
fn same_id_contradiction_is_checked_before_event_filtering_or_class_splitting() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let [first, _] = reports(&fixture, cases.first().ok_or("ordinary case")?)?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&first))?;
    let mut dataset = ExportDataset::load(&store)?;
    let mut excluded = first.event.clone();
    excluded.id = Id::mint("event", &["excluded-event"]);
    excluded.kind = EventKind::Unmapped {
        label: "100m Wheelchair".into(),
    };
    excluded
        .source_labels
        .first_mut()
        .ok_or("source label")?
        .label = "100m Wheelchair".into();
    let mut admitted = first.event.clone();
    admitted.id = Id::mint("event", &["different-admitted-event"]);
    admitted.kind = EventKind::Track200m;
    admitted
        .source_labels
        .first_mut()
        .ok_or("source label")?
        .label = "200 Meter Dash".into();
    dataset.events.extend([excluded.clone(), admitted.clone()]);
    let admission = crate::report::EventAdmission::of(&dataset)?;
    check!(admission.included_kind(&excluded.id).is_none());
    check!(admission.included_kind(&admitted.id).is_some());
    for event in [
        Id::mint("event", &["unknown-event"]),
        excluded.id,
        admitted.id,
    ] {
        let mut other = first.performance.clone();
        other.event = event;
        dataset.performances = vec![first.performance.clone(), other];
        for _ in 0..2 {
            let error = rows(&mut dataset, 21)
                .err()
                .ok_or("event contradiction accepted")?;
            check!(error
                .to_string()
                .contains("incompatible retained identity or context"));
            dataset.performances.reverse();
        }
    }
    Ok(())
}

#[test]
fn equivalent_conversion_candidates_keep_deterministic_best_adjusted_value() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let case = cases.first().ok_or("ordinary case")?;
    let [first, _] = reports(&fixture, case)?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), std::slice::from_ref(&first))?;
    let mut dataset = ExportDataset::load(&store)?;
    let mut slower = first.performance;
    let original = Mark::TimeSeconds(ExactSeconds::parse("51.00")?);
    slower.adjustment = Some(Adjustment::track_size_conversion(
        Some(original.clone()),
        slower.mark.clone(),
        Some("first derivation".into()),
        None,
    ));
    let mut faster = slower.clone();
    faster.mark = case.better.clone();
    faster.adjustment = Some(Adjustment::track_size_conversion(
        Some(original.clone()),
        faster.mark.clone(),
        Some("second derivation".into()),
        Some(SourceRef::new("timer", None)),
    ));
    dataset.performances = vec![slower, faster];
    for _ in 0..2 {
        let selected = single(&mut dataset, 21)?;
        check!(eq; selected.result.mark, case.better);
        check!(eq; selected.result.original_mark, Some(original.clone()));
        check!(eq; selected.inventory.eligible_group_count(), 1);
        check!(selected.conflicts.is_empty());
        dataset.performances.reverse();
    }
    Ok(())
}

#[test]
fn equal_total_but_different_combined_completion_conflicts_only_within_exact_id() -> TestResult {
    let fixture = TieFixture::new()?;
    let cases = numeric_cases()?;
    let [first, clean] = reports(&fixture, cases.last().ok_or("combined case")?)?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), &[first.clone(), clean.clone()])?;
    let mut dataset = ExportDataset::load(&store)?;
    let mut changed = first.performance.clone();
    let completion = changed.combined_completion.as_ref().ok_or("completion")?;
    let mut components = completion.components().to_vec();
    *components.first_mut().ok_or("first component")? =
        CombinedComponentResult::Completed(CentiPoints::new(50_001));
    *components.get_mut(1).ok_or("second component")? =
        CombinedComponentResult::Completed(CentiPoints::new(49_999));
    changed.combined_completion = Some(CombinedEventCompletion::new(
        first
            .event
            .specification
            .combined_event
            .clone()
            .ok_or("specification")?,
        components,
    )?);
    check!(changed.combined_completion != first.performance.combined_completion);
    dataset.performances = vec![
        first.performance.clone(),
        changed,
        clean.performance.clone(),
    ];
    for _ in 0..2 {
        let selected = single(&mut dataset, 21)?;
        check!(eq; selected.source.performance_id, clean.performance.id);
        check!(eq; selected.inventory.eligible_group_count(), 1);
        check!(eq; selected.conflicts.len(), 1);
        check!(eq; single(&mut dataset, 20)?.inventory.eligible_group_count(), 1);
        dataset.performances.reverse();
    }
    Ok(())
}
