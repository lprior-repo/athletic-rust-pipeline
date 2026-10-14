use super::*;

fn assert_complete_winner(reports: &[Observation], complete: &CanonicalPerformance) -> TestResult {
    for order in orders(reports)? {
        assert_selection(&order, 20, complete, 2)?;
    }
    Ok(())
}

#[test]
fn combined_context_missing_or_unknown_completion_does_not_quarantine_known_total() -> TestResult {
    let [complete, unknown, clean] = context_reports(CombinedComponentResult::Unknown)?;
    assert_complete_winner(
        &[complete.clone(), unknown.clone(), clean.clone()],
        &complete.performance,
    )?;
    let mut missing = unknown;
    missing.performance.combined_completion = None;
    assert_complete_winner(&[complete.clone(), missing, clean], &complete.performance)
}

#[test]
fn combined_context_ambiguous_or_unknown_raw_status_does_not_quarantine_known_total() -> TestResult
{
    for raw in ["X", " -- ", "unknown", "DQ.", "DNF 10.941"] {
        let [complete, mut unknown, clean] = context_reports(CombinedComponentResult::Unknown)?;
        unknown.performance.mark = Mark::Raw(raw.into());
        unknown.performance.combined_completion = None;
        assert_complete_winner(&[complete.clone(), unknown, clean], &complete.performance)?;
    }
    Ok(())
}

#[test]
fn combined_context_explicit_raw_status_uses_clean_fallback_in_both_orders() -> TestResult {
    for raw in ["DNF", " DNS ", "dq", "NM"] {
        let [complete, mut terminal, clean] = context_reports(CombinedComponentResult::Unknown)?;
        terminal.performance.mark = Mark::Raw(raw.into());
        terminal.performance.combined_completion = None;
        assert_clean_winner(&[complete.clone(), terminal.clone(), clean.clone()])?;
        assert_clean_winner(&[terminal, complete, clean])?;
    }
    Ok(())
}

#[test]
fn combined_context_foreign_scoring_completion_does_not_quarantine_known_total() -> TestResult {
    let [complete, mut terminal, clean] = context_reports(CombinedComponentResult::DidNotFinish)?;
    let components = terminal
        .performance
        .combined_completion
        .as_ref()
        .ok_or("missing foreign scoring components")?
        .components()
        .to_vec();
    terminal.performance.combined_completion = Some(CombinedEventCompletion::new(
        pentathlon_specification(2)?,
        components,
    )?);
    assert_complete_winner(&[complete.clone(), terminal, clean], &complete.performance)
}

#[test]
fn combined_context_missing_event_specification_does_not_quarantine_known_class() -> TestResult {
    let fixture = TieFixture::new()?;
    let [complete, _, clean] = context_reports(CombinedComponentResult::Unknown)?;
    let missing = in_same_meet(
        pentathlon_report(&fixture, "context-missing-spec", 500000, None)?,
        &complete.meet,
    )?;
    assert_complete_winner(&[complete.clone(), missing, clean], &complete.performance)
}

#[test]
fn combined_context_explicit_no_mark_component_uses_clean_fallback() -> TestResult {
    let [complete, terminal, clean] = context_reports(CombinedComponentResult::NoMark)?;
    assert_clean_winner(&[complete.clone(), terminal.clone(), clean.clone()])?;
    assert_clean_winner(&[terminal, complete, clean])
}

#[test]
fn combined_context_independent_context_does_not_poison_completed_total() -> TestResult {
    for coordinate in ["round", "heat", "date", "meet"] {
        let [complete, mut terminal, clean] =
            context_reports(CombinedComponentResult::DidNotFinish)?;
        match coordinate {
            "round" => terminal.performance.round = Some("Prelim".into()),
            "heat" => terminal.performance.heat = Some("2".into()),
            "date" => terminal.performance.date = "2026-04-03".into(),
            "meet" => terminal = in_same_meet(terminal, &clean.meet)?,
            _ => return Err("unknown context coordinate".into()),
        }
        assert_complete_winner(&[complete.clone(), terminal, clean], &complete.performance)?;
    }
    Ok(())
}

#[test]
fn combined_context_contested_equal_total_is_not_a_tied_winner() -> TestResult {
    let [complete, terminal, mut clean] = context_reports(CombinedComponentResult::DidNotFinish)?;
    clean = clean.with_mark(complete.performance.mark.clone())?;
    assert_clean_winner(&[complete.clone(), terminal.clone(), clean.clone()])?;
    assert_clean_winner(&[terminal, complete, clean])
}
