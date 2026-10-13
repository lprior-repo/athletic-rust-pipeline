use super::*;
use crate::bests::Conflict;
use census_domain::model::RetainedConflict;

fn indoor_400() -> TestResult<ExportDataset> {
    let mut dataset = selection_dataset(EventKind::Track400m)?;
    dataset.meets[0].sports = vec![Sport::IndoorTrack];
    Ok(dataset)
}

#[test]
fn equal_originals_keep_adjusted_reports_uncontested_and_select_the_better_conversion() -> TestResult
{
    let mut dataset = indoor_400()?;
    let slower = converted(
        &dataset,
        "equal_original_slow",
        Some(seconds("51.00")?),
        seconds("50.00")?,
    );
    let faster = converted(
        &dataset,
        "equal_original_fast",
        Some(seconds("51.00")?),
        seconds("49.00")?,
    );
    dataset.performances = vec![slower, faster.clone()];
    let original = dataset.performances.clone();
    let rows = selected(&dataset);
    check!(eq; dataset.performances, original);
    dataset.performances.reverse();
    check!(eq; selected(&dataset), rows);
    check!(eq; rows.len(), 1);
    check!(eq; rows[0].source.performance_id, faster.id);
    check!(eq; rows[0].result.value, 49_000_000_000);
    check!(eq; rows[0].result.mark, seconds("49.00")?);
    check!(eq; rows[0].result.original_mark, Some(seconds("51.00")?));
    check!(eq; rows[0].population.marks, 2);
    check!(eq; rows[0].population.sources, 1);
    check!(eq; rows[0].conflicts, Vec::new());
    Ok(())
}

#[test]
fn different_originals_contest_equal_adjusted_reports_and_allow_only_a_clean_context() -> TestResult
{
    let mut dataset = indoor_400()?;
    let first = converted(
        &dataset,
        "different_original_first",
        Some(seconds("50.00")?),
        seconds("49.00")?,
    );
    let second = converted(
        &dataset,
        "different_original_second",
        Some(seconds("51.00")?),
        seconds("49.00")?,
    );
    dataset.performances = vec![first, second];
    check!(eq; selected(&dataset), Vec::new());
    dataset.performances.reverse();
    check!(eq; selected(&dataset), Vec::new());
    let mut fallback = converted(
        &dataset,
        "different_original_fallback",
        Some(seconds("53.00")?),
        seconds("52.00")?,
    );
    fallback.date = "2025-03-02".into();
    dataset.performances.push(fallback.clone());
    let rows = selected(&dataset);
    dataset.performances.reverse();
    check!(eq; selected(&dataset), rows);
    check!(eq; rows.len(), 1);
    check!(eq; rows[0].source.performance_id, fallback.id);
    check!(eq; rows[0].result.value, 52_000_000_000);
    check!(eq; rows[0].result.original_mark, Some(seconds("53.00")?));
    check!(eq; rows[0].population.marks, 3);
    check!(eq; rows[0].population.sources, 1);
    check!(eq; rows[0].conflicts, vec![Conflict { meet: "Test Meet".into(), marks: vec!["49.00".into()] }]);
    Ok(())
}

#[test]
fn missing_or_nonnumeric_originals_compare_adjusted_candidate_values() -> TestResult {
    [
        None,
        Some(Mark::Raw("published original unavailable".into())),
    ]
    .into_iter()
    .try_for_each(|original| -> TestResult {
        let mut dataset = indoor_400()?;
        let first = converted(
            &dataset,
            "fallback_original_first",
            original.clone(),
            seconds("50.00")?,
        );
        let second = converted(
            &dataset,
            "fallback_original_second",
            original,
            seconds("49.00")?,
        );
        dataset.performances = vec![first, second];
        check!(eq; selected(&dataset), Vec::new());
        dataset.performances.reverse();
        check!(eq; selected(&dataset), Vec::new());
        Ok(())
    })
}

fn poisoned_actual(dataset: &ExportDataset) -> TestResult<CanonicalPerformance> {
    let mut actual = reported(dataset, "retained_adjustment_context", seconds("51.00")?);
    actual.retained_conflicts.push(RetainedConflict::new(
        "Individual result scalar",
        actual.id.as_str(),
        "mark, timing, place, observed_grade",
        "retained contradictory original 51.00 versus 52.00",
    ));
    Ok(actual)
}

#[test]
fn retained_context_conflict_poisoning_crosses_actual_and_adjusted_keys_and_clean_duplicates(
) -> TestResult {
    let mut dataset = indoor_400()?;
    let actual = poisoned_actual(&dataset)?;
    let duplicate = reported(&dataset, "retained_adjustment_duplicate", seconds("51.00")?);
    let conversion = converted(
        &dataset,
        "retained_adjustment_conversion",
        Some(seconds("51.00")?),
        seconds("50.00")?,
    );
    dataset.performances = vec![actual, duplicate, conversion];
    let original = dataset.performances.clone();
    check!(eq; selected(&dataset), Vec::new());
    check!(eq; dataset.performances, original);
    dataset.performances.reverse();
    check!(eq; selected(&dataset), Vec::new());
    Ok(())
}

#[derive(Clone, Copy)]
enum SeparateContext {
    Date,
    Round,
    Heat,
    Meet,
    Event,
}

fn separate_context(
    dataset: &mut ExportDataset,
    clean: &mut CanonicalPerformance,
    context: SeparateContext,
) -> TestResult {
    match context {
        SeparateContext::Date => clean.date = "2025-03-02".into(),
        SeparateContext::Round => clean.round = Some("Final".into()),
        SeparateContext::Heat => clean.heat = Some("2".into()),
        SeparateContext::Meet => {
            let mut meet = dataset.meets[0].clone();
            meet.id = Id::mint("meet", &["clean_adjustment_other_meet"]);
            let event = test_event(EventKind::Track400m, &meet.id)?;
            clean.meet = meet.id.clone();
            clean.event = event.id.clone();
            dataset.meets.push(meet);
            dataset.events.push(event);
        }
        SeparateContext::Event => {
            let mut event = dataset.events[0].clone();
            event.id = Id::mint("evt", &["clean_adjustment_other_event"]);
            clean.event = event.id.clone();
            dataset.events.push(event);
        }
    }
    Ok(())
}

#[test]
fn retained_context_conflict_leaves_distinct_converted_competition_contexts_eligible() -> TestResult
{
    [SeparateContext::Date, SeparateContext::Round, SeparateContext::Heat, SeparateContext::Meet, SeparateContext::Event]
        .into_iter()
        .try_for_each(|context| -> TestResult {
            let mut dataset = indoor_400()?;
            let actual = poisoned_actual(&dataset)?;
            let contested = converted(&dataset, "poisoned_conversion", Some(seconds("51.00")?), seconds("50.00")?);
            let mut clean = converted(&dataset, "clean_conversion_context", Some(seconds("53.00")?), seconds("52.00")?);
            separate_context(&mut dataset, &mut clean, context)?;
            dataset.performances = vec![actual, contested, clean.clone()];
            let rows = selected(&dataset);
            dataset.performances.reverse();
            check!(eq; selected(&dataset), rows);
            check!(eq; rows.len(), 1);
            check!(eq; rows[0].source.performance_id, clean.id);
            check!(eq; rows[0].result.value, 52_000_000_000);
            check!(eq; rows[0].result.original_mark, Some(seconds("53.00")?));
            check!(eq; rows[0].population.marks, 2);
            check!(eq; rows[0].conflicts, vec![Conflict { meet: "Test Meet".into(), marks: vec!["50.00".into()] }]);
            Ok(())
        })
}
