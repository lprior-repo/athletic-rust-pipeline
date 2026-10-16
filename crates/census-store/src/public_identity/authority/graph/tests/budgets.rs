use super::*;
use crate::read::ReadBudget;
use census_domain::model::{
    Grade, ObservedGrade, SchoolYear, SourceAthleteObservation, SourceNamespace, SourceRef,
};

fn observation() -> TestResult<SourceObservation> {
    let grade = ObservedGrade {
        grade: Grade::new(11).ok_or("grade")?,
        school_year: SchoolYear::new(2025).ok_or("school year")?,
        source: SourceRef::new("graph enrichment", None),
    };
    Ok(SourceObservation::Athlete(
        SourceAthleteObservation::new(
            SourceNamespace::MilesplitAthlete,
            "51003",
            "graph-observation",
            "Runner",
            "2026-10-15",
        )
        .with_grade(Some(grade)),
    ))
}

#[test]
fn enrichment_owners_are_cumulative_across_batches() -> TestResult {
    let mut admission = RetainedEnrichment::new(ReadBudget { rows: 2, ..BUDGET });
    for index in 0..2 {
        let mut batch = [subject(&format!("owners-{index}"), "51003")?];
        admission.enrich(&mut batch, |_| Ok(()))?;
    }
    let mut extra = [subject("owners-extra", "51003")?];
    check!(admission.enrich(&mut extra, |_| Ok(())).is_err());
    check!(admission.enrich(&mut [], |_| Ok(())).is_err());
    Ok(())
}

#[test]
fn enrichment_grade_bytes_are_cumulative_and_failed_batch_rolls_back() -> TestResult {
    let row = observation()?;
    let SourceObservation::Athlete(observation) = &row else {
        return Err("athlete observation".into());
    };
    let grade = observation.observed_grade.as_ref().ok_or("grade")?;
    let bytes = u64::try_from(serde_json::to_vec(grade)?.len())?;
    let mut admission = RetainedEnrichment::new(ReadBudget {
        rows: 4,
        bytes,
        ..BUDGET
    });
    let mut first = [subject("grade-first", "51003")?];
    admission.enrich(&mut first, |batch| batch.accept(&row))?;
    check!(eq; first.first().ok_or("first")?.observed_grades.len(), 1);
    let mut second = [subject("grade-second", "51003")?];
    check!(admission
        .enrich(&mut second, |batch| batch.accept(&row))
        .is_err());
    check!(second.first().ok_or("second")?.observed_grades.is_empty());
    check!(admission.enrich(&mut second, |_| Ok(())).is_err());
    Ok(())
}

#[test]
fn physical_row_boundary_is_not_reset_by_merge() -> TestResult {
    for count in [8192_usize, 8193] {
        let (_directory, store, writer) = enrolled()?;
        let athlete = subject("physical-boundary", "51004")?;
        store.append_many(Table::Athletes, &vec![athlete.clone(); count])?;
        let outcome = writer.capture_raw_generation_v2(
            &store,
            &RawGenerationOperationIdV2::new("physical-boundary")?,
        )?;
        let held = store.snapshot();
        let retained = outcome.retained();
        let graph = VerifiedRawGraph::verify(&held, retained.binding(), retained.seal())?;
        let actual = accept_source(&graph, &athlete.id.cast(), "2026-10-15");
        if count == 8192 {
            check!(actual.is_ok());
        } else {
            check!(actual.is_err());
        }
    }
    Ok(())
}

#[test]
fn enriched_decision_matches_historical_inputs() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    let athlete = subject("enriched-graph", "51003")?;
    let row = observation()?;
    store.append(Table::Athletes, &athlete)?;
    store.append_many(Table::SourceObservations, &[row.clone(), row])?;
    compare(&store, &writer, &athlete.id.cast())
}

#[test]
fn duplicate_and_preexisting_grades_do_not_recharge_admission() -> TestResult {
    let row = observation()?;
    let SourceObservation::Athlete(observation) = &row else {
        return Err("athlete observation".into());
    };
    let grade = observation.observed_grade.as_ref().ok_or("grade")?.clone();
    let mut athlete = subject("preexisting-grade", "51003")?;
    athlete.observed_grades.push(grade);
    let mut batch = [athlete];
    let mut admission = RetainedEnrichment::new(ReadBudget {
        rows: 1,
        bytes: 0,
        ..BUDGET
    });
    admission.enrich(&mut batch, |targets| {
        targets.accept(&row)?;
        targets.accept(&row)
    })?;
    check!(eq; batch.first().ok_or("athlete")?.observed_grades.len(), 1);
    Ok(())
}

#[test]
fn later_scan_failure_rolls_back_and_poisoned_budget_cannot_resume() -> TestResult {
    let row = observation()?;
    let mut batch = [subject("late-failure", "51003")?];
    let mut admission = RetainedEnrichment::new(BUDGET);
    check!(admission
        .enrich(&mut batch, |targets| {
            targets.accept(&row)?;
            Err(StoreError::CounterOverflow)
        })
        .is_err());
    check!(batch.first().ok_or("athlete")?.observed_grades.is_empty());
    check!(admission.enrich(&mut batch, |_| Ok(())).is_err());
    Ok(())
}

#[test]
fn empty_athletes_do_not_skip_observation_budget() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    let candidate = subject("empty-athletes", "51003")?.id.cast();
    store.append_many(Table::SourceObservations, &vec![observation()?; 8193])?;
    let outcome = writer
        .capture_raw_generation_v2(&store, &RawGenerationOperationIdV2::new("empty-athletes")?)?;
    let held = store.snapshot();
    let retained = outcome.retained();
    let graph = VerifiedRawGraph::verify(&held, retained.binding(), retained.seal())?;
    check!(matches!(
        accept_source(&graph, &candidate, "2026-10-15"),
        Err(StoreError::BudgetExceeded { .. })
    ));
    Ok(())
}

#[test]
fn enrichment_addition_rows_do_not_reset_across_batches() -> TestResult {
    let first_row = observation()?;
    let mut second_row = first_row.clone();
    let SourceObservation::Athlete(second) = &mut second_row else {
        return Err("athlete observation".into());
    };
    second.observed_grade.as_mut().ok_or("grade")?.grade = Grade::new(12).ok_or("grade12")?;
    let mut admission = RetainedEnrichment::new(ReadBudget { rows: 3, ..BUDGET });
    let mut first = [subject("additions-first", "51003")?];
    admission.enrich(&mut first, |targets| {
        targets.accept(&first_row)?;
        targets.accept(&second_row)
    })?;
    let mut second = [subject("additions-second", "51003")?];
    check!(admission
        .enrich(&mut second, |targets| {
            targets.accept(&first_row)?;
            targets.accept(&second_row)
        })
        .is_err());
    check!(second.first().ok_or("second")?.observed_grades.is_empty());
    Ok(())
}

#[test]
fn swallowed_batch_error_cannot_resume_or_report_success() -> TestResult {
    let row = observation()?;
    let mut batch = [subject("swallowed-failure", "51003")?];
    let mut admission = RetainedEnrichment::new(ReadBudget { bytes: 0, ..BUDGET });
    check!(admission
        .enrich(&mut batch, |targets| {
            if targets.accept(&row).is_ok() {
                return Err(StoreError::CounterOverflow);
            }
            Ok(())
        })
        .is_err());
    check!(admission.enrich(&mut batch, |_| Ok(())).is_err());
    check!(batch.first().ok_or("athlete")?.observed_grades.is_empty());
    Ok(())
}

#[test]
fn enriched_decision_in_second_batch_matches_historical_digest() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    let SourceObservation::Athlete(template) = observation()? else {
        return Err("athlete observation".into());
    };
    let mut athletes = Vec::new();
    let mut observations = Vec::new();
    for index in 0..257 {
        let source_id = format!("52{index:04}");
        athletes.push(subject(&format!("enriched-batch-{index}"), &source_id)?);
        observations.push(SourceObservation::Athlete(
            SourceAthleteObservation::new(
                SourceNamespace::MilesplitAthlete,
                source_id,
                format!("observation-{index}"),
                "Runner",
                "2026-10-15",
            )
            .with_grade(template.observed_grade.clone()),
        ));
    }
    athletes.sort_by(|left, right| left.id.cmp(&right.id));
    let candidate = athletes.last().ok_or("second-batch athlete")?.id.cast();
    store.append_many(Table::Athletes, &athletes)?;
    store.append_many(Table::SourceObservations, &observations)?;
    compare(&store, &writer, &candidate)
}
