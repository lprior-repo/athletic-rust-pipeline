use super::*;

fn timing_pair(dataset: &ExportDataset) -> TestResult<[CanonicalPerformance; 2]> {
    let first = timed_report(dataset, "retained_timing_conflict", "50.00")?;
    let mut revision = first.clone();
    revision.mark = Mark::TimeSeconds(ExactSeconds::parse("51.00")?);
    revision.timing = Some(TimingMethod::Hand);
    Ok([first, revision])
}

fn wind_pair(dataset: &ExportDataset) -> [CanonicalPerformance; 2] {
    let first = distance_report(dataset, "retained_wind_class_conflict", 800);
    let mut revision = first.clone();
    revision.mark = Mark::DistanceMetres(CentiMetres::new(750));
    revision.wind_mps = Some(3.0);
    [first, revision]
}

fn duplicate_of(performance: &CanonicalPerformance) -> CanonicalPerformance {
    let mut duplicate = performance.clone();
    duplicate.id = Id::mint("perf", &["duplicate_in_other_comparison_class"]);
    duplicate.source_key = "duplicate_in_other_comparison_class".into();
    duplicate.source_athlete = Some(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "2001"));
    duplicate
}

fn assert_class_quarantine(
    seed: &ExportDataset,
    pair: &[CanonicalPerformance; 2],
    fallback_mark: Mark,
    expected: i64,
) -> TestResult {
    append_orders(pair)
        .into_iter()
        .try_for_each(|order| -> TestResult {
            let duplicate = duplicate_of(&order[1]);
            let observations = [order[0].clone(), order[1].clone(), duplicate.clone()];
            let mut dataset = reloaded_dataset(seed, &observations)?;
            retained_conflict(&dataset, &pair[0].id)?;
            check!(eq; selected(&dataset)?, Vec::new());
            dataset.performances.reverse();
            check!(eq; selected(&dataset)?, Vec::new());

            let mut clean = duplicate;
            clean.id = Id::mint("perf", &["clean_other_context_comparison_class"]);
            clean.source_key = "clean_other_context_comparison_class".into();
            clean.date = "2025-02-28".into();
            clean.mark = fallback_mark.clone();
            let observations = [
                observations[0].clone(),
                observations[1].clone(),
                observations[2].clone(),
                clean.clone(),
            ];
            let mut dataset = reloaded_dataset(seed, &observations)?;
            let rows = selected(&dataset)?;
            dataset.performances.reverse();
            check!(eq; selected(&dataset)?, rows);
            check!(eq; rows.len(), 1);
            check!(eq; rows[0].source.performance_id, clean.id);
            check!(eq; rows[0].result.value, expected);
            Ok(())
        })
}

#[test]
fn retained_timing_conflict_cannot_resurrect_in_another_timing_class() -> TestResult {
    let seed = selection_dataset(EventKind::Track400m)?;
    assert_class_quarantine(
        &seed,
        &timing_pair(&seed)?,
        Mark::TimeSeconds(ExactSeconds::parse("52.00")?),
        52_000_000_000,
    )
}

#[test]
fn retained_numeric_conflict_cannot_resurrect_in_another_wind_class() -> TestResult {
    let seed = selection_dataset(EventKind::LongJump)?;
    assert_class_quarantine(
        &seed,
        &wind_pair(&seed),
        Mark::DistanceMetres(CentiMetres::new(700)),
        7_000_000,
    )
}

#[test]
fn unresolvable_retained_conflict_still_quarantines_comparable_duplicates() -> TestResult {
    let seed = selection_dataset(EventKind::Track400m)?;
    let mut pair = timing_pair(&seed)?;
    pair[0].mark = Mark::Raw("unresolved published mark".into());
    append_orders(&pair)
        .into_iter()
        .try_for_each(|order| -> TestResult {
            let duplicate = duplicate_of(&pair[1]);
            let observations = [order[0].clone(), order[1].clone(), duplicate.clone()];
            let mut dataset = reloaded_dataset(&seed, &observations)?;
            retained_conflict(&dataset, &pair[0].id)?;
            check!(eq; selected(&dataset)?, Vec::new());
            dataset.performances.reverse();
            check!(eq; selected(&dataset)?, Vec::new());

            let mut clean = duplicate;
            clean.id = Id::mint("perf", &["clean_after_unresolvable_conflict"]);
            clean.source_key = "clean_after_unresolvable_conflict".into();
            clean.date = "2025-02-28".into();
            clean.mark = Mark::TimeSeconds(ExactSeconds::parse("52.00")?);
            let observations = [
                observations[0].clone(),
                observations[1].clone(),
                observations[2].clone(),
                clean.clone(),
            ];
            let dataset = reloaded_dataset(&seed, &observations)?;
            let rows = selected(&dataset)?;
            check!(eq; rows.len(), 1);
            check!(eq; rows[0].source.performance_id, clean.id);
            check!(eq; rows[0].result.value, 52_000_000_000);
            Ok(())
        })
}

#[test]
fn retained_conflict_does_not_quarantine_another_athlete_in_the_same_context() -> TestResult {
    let mut seed = selection_dataset(EventKind::Track400m)?;
    let pair = timing_pair(&seed)?;
    let original = seed.athletes.first().ok_or("fixture athlete")?;
    let mut other = CanonicalAthlete::new(
        &original.school,
        "Different synthetic runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1002"),
    );
    other.published_graduations = original.published_graduations.clone();
    let mut clean = duplicate_of(&pair[1]);
    clean.athlete = other.id.clone();
    clean.source_athlete = other.source.clone();
    seed.athletes.push(other);
    append_orders(&pair)
        .into_iter()
        .try_for_each(|order| -> TestResult {
            let observations = [order[0].clone(), order[1].clone(), clean.clone()];
            let mut dataset = reloaded_dataset(&seed, &observations)?;
            let rows = selected(&dataset)?;
            dataset.performances.reverse();
            check!(eq; selected(&dataset)?, rows);
            check!(eq; rows.len(), 1);
            check!(eq; rows[0].source.performance_id, clean.id);
            check!(eq; rows[0].result.value, 51_000_000_000);
            Ok(())
        })
}
