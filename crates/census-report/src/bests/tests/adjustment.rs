use super::*;
use crate::bests::{ComparisonPolicy, SurfaceClass};
use census_domain::model::{Adjustment, AdjustmentBasis, AdjustmentKind};

const TRACK_SIZE_NOTE: &str = "Converted from 9:22.21 for Track Size. ";

fn seconds(text: &str) -> TestResult<Mark> {
    Ok(Mark::TimeSeconds(ExactSeconds::parse_clock(text)?))
}

fn converted(
    dataset: &ExportDataset,
    label: &str,
    original: Option<Mark>,
    adjusted: Mark,
) -> CanonicalPerformance {
    let mut performance = reported(dataset, label, adjusted);
    performance.adjustment = Some(Adjustment::track_size_conversion(
        original,
        performance.mark.clone(),
        Some(TRACK_SIZE_NOTE.to_string()),
        None,
    ));
    performance
}

#[test]
fn selection_keeps_conversion_and_actual_reports_of_one_race_distinct() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track3200m)?;
    dataset.meets[0].sports = vec![Sport::IndoorTrack];
    let actual = reported(&dataset, "actual", seconds("9:22.21")?);
    let adjusted_mark = seconds("9:15.75")?;
    let conversion = converted(
        &dataset,
        "converted",
        Some(seconds("9:22.21")?),
        adjusted_mark.clone(),
    );
    dataset.performances = vec![actual.clone(), conversion.clone()];

    let rows = selected(&dataset);

    check!(eq; rows.len(), 2);
    let converted_row = rows
        .iter()
        .find(|row| row.source.performance_id == conversion.id)
        .ok_or("the converted report is selected on its own key")?;
    let actual_row = rows
        .iter()
        .find(|row| row.source.performance_id == actual.id)
        .ok_or("the actual report is selected beside it")?;
    check!(ne; converted_row.key, actual_row.key);
    check!(eq; converted_row.key.surface, SurfaceClass::Indoor);
    check!(eq; converted_row.key.comparison, ComparisonPolicy::Standard);
    check!(eq; converted_row.result.mark, adjusted_mark);
    check!(eq; converted_row.result.original_mark, Some(seconds("9:22.21")?));
    check!(
        eq;
        converted_row.result.adjustment_kind,
        Some(AdjustmentKind::TrackSizeConversion)
    );
    check!(
        eq;
        converted_row.result.adjustment_basis,
        Some(AdjustmentBasis::Known)
    );
    check!(eq; converted_row.population.marks, 1);
    check!(eq; converted_row.conflicts, Vec::new());
    check!(eq; actual_row.result.original_mark, None);
    check!(eq; actual_row.result.adjustment_kind, None);
    check!(eq; actual_row.result.adjustment_basis, None);
    check!(eq; actual_row.population.marks, 1);
    check!(eq; actual_row.conflicts, Vec::new());
    Ok(())
}

#[test]
fn unknown_conversion_basis_is_unqualified_and_kept_apart_from_known_conversions() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track3200m)?;
    dataset.meets[0].sports = vec![Sport::IndoorTrack];
    let unknown = converted(&dataset, "unknown_conversion", None, seconds("9:15.75")?);
    dataset.performances = vec![unknown.clone()];

    let rows = selected(&dataset);

    check!(eq; rows.len(), 1);
    check!(eq; rows[0].key.comparison, ComparisonPolicy::UnqualifiedAdjustment);
    check!(
        eq;
        rows[0].result.adjustment_basis,
        Some(AdjustmentBasis::Unknown)
    );
    check!(eq; rows[0].result.original_mark, None);

    let known = converted(
        &dataset,
        "known_conversion",
        Some(seconds("9:22.21")?),
        seconds("9:15.75")?,
    );
    dataset.performances = vec![unknown, known];

    let rows = selected(&dataset);

    check!(eq; rows.len(), 2);
    check!(ne; rows[0].key, rows[1].key);
    check!(
        eq;
        rows.iter()
            .filter(|row| row.key.comparison == ComparisonPolicy::UnqualifiedAdjustment)
            .count(),
        1
    );
    check!(
        eq;
        rows.iter()
            .filter(|row| row.key.comparison == ComparisonPolicy::Standard)
            .count(),
        1
    );
    Ok(())
}

#[test]
fn contested_actuals_still_conflict_beside_a_conversion() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track3200m)?;
    dataset.meets[0].sports = vec![Sport::IndoorTrack];
    let slower = reported(&dataset, "actual_slower", seconds("9:22.21")?);
    let faster = reported(&dataset, "actual_faster", seconds("9:20.00")?);
    let conversion = converted(
        &dataset,
        "converted",
        Some(seconds("9:20.00")?),
        seconds("9:15.75")?,
    );
    dataset.performances = vec![slower.clone(), faster.clone(), conversion.clone()];

    let forward = selected(&dataset);
    dataset.performances.reverse();
    check!(eq; selected(&dataset), forward);

    check!(eq; forward.len(), 1);
    let converted_row = forward
        .iter()
        .find(|row| row.source.performance_id == conversion.id)
        .ok_or("the converted report keeps its own uncontested slot")?;
    check!(eq; converted_row.result.mark, seconds("9:15.75")?);
    check!(
        eq; converted_row.result.original_mark, Some(seconds("9:20.00")?)
    );
    check!(eq; converted_row.conflicts, Vec::new());
    check!(eq; converted_row.population.marks, 1);
    check!(
        forward
            .iter()
            .all(|row| row.source.performance_id != faster.id
                && row.source.performance_id != slower.id),
        "the contested raw actuals are withheld beside the conversion"
    );
    Ok(())
}

#[test]
fn a_conversion_elsewhere_stays_bound_to_its_own_meet_and_source() -> TestResult {
    let mut dataset = selection_dataset(EventKind::Track3200m)?;
    dataset.meets[0].sports = vec![Sport::IndoorTrack];
    let mut other_meet = dataset.meets[0].clone();
    other_meet.id = Id::mint("meet", &["conversion_host_meet"]);
    other_meet.name = "Conversion Host".to_string();
    other_meet.normalized_name = "conversion_host".to_string();
    other_meet.date = "2025-02-15".to_string();
    let other_event = test_event(EventKind::Track3200m, &other_meet.id)?;
    let mut conversion = converted(
        &dataset,
        "converted_elsewhere",
        Some(seconds("9:22.21")?),
        seconds("9:15.75")?,
    );
    conversion.meet = other_meet.id.clone();
    conversion.event = other_event.id.clone();
    conversion.date = other_meet.date.clone();
    let actual = reported(&dataset, "actual", seconds("9:22.21")?);
    dataset.meets.push(other_meet.clone());
    dataset.events.push(other_event);
    dataset.performances = vec![actual, conversion.clone()];

    let rows = selected(&dataset);

    check!(eq; rows.len(), 2);
    let converted_row = rows
        .iter()
        .find(|row| row.source.performance_id == conversion.id)
        .ok_or("the converted report is selected at its own meet")?;
    check!(eq; converted_row.meet.meet_id, other_meet.id);
    check!(eq; converted_row.meet.date, other_meet.date);
    check!(eq; converted_row.result.original_mark, Some(seconds("9:22.21")?));
    check!(eq; converted_row.conflicts, Vec::new());
    Ok(())
}
