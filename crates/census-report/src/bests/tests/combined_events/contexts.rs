use super::*;

#[path = "historical.rs"]
mod historical;
#[path = "precision.rs"]
mod precision;

fn context_reports(status: CombinedComponentResult) -> TestResult<[Observation; 3]> {
    let fixture = TieFixture::new()?;
    let complete = pentathlon_report(
        &fixture,
        "context-complete",
        500000,
        Some(pentathlon_specification(1)?),
    )?;
    let mut terminal = in_same_meet(
        pentathlon_report(
            &fixture,
            "context-terminal",
            500000,
            Some(pentathlon_specification(1)?),
        )?,
        &complete.meet,
    )?;
    terminal.performance.combined_completion = Some(CombinedEventCompletion::new(
        pentathlon_specification(1)?,
        vec![
            CombinedComponentResult::Completed(CentiPoints::new(100000)),
            CombinedComponentResult::Completed(CentiPoints::new(100000)),
            CombinedComponentResult::Completed(CentiPoints::new(100000)),
            status,
            CombinedComponentResult::Completed(CentiPoints::new(100000)),
        ],
    )?);
    let mut clean = pentathlon_report(
        &fixture,
        "context-clean",
        400000,
        Some(pentathlon_specification(1)?),
    )?;
    clean.performance.date = "2026-04-02".into();
    Ok([complete, terminal, clean])
}

fn assert_selection(
    reports: &[Observation],
    policy: u32,
    expected: &CanonicalPerformance,
    eligible: usize,
) -> TestResult {
    let fixture = TieFixture::new()?;
    let directory = tempfile::tempdir()?;
    let store = fixture.store_with_replay(directory.path(), reports)?;
    let mut dataset = ExportDataset::load(&store)?;
    dataset.lineage.policy_revision = policy;
    let rows = selected(&dataset)?;
    check!(eq; rows.len(), 1);
    let row = rows.first().ok_or("missing expected winner")?;
    check!(eq; row.source.performance_id, expected.id);
    check!(eq; row.result.mark, expected.mark);
    check!(eq; row.inventory.eligible_group_count(), eligible);
    check!(eq; row.inventory.tied_group_count(), 1);
    check!(eq; row.inventory.tied_representative_ids(), vec![expected.id.clone()]);
    Ok(())
}

fn assert_clean_winner(reports: &[Observation]) -> TestResult {
    let clean = reports.last().ok_or("missing independent fallback")?;
    assert_selection(reports, 20, &clean.performance, 1)
}

#[test]
fn combined_context_terminal_after_completed_uses_clean_fallback() -> TestResult {
    for status in [
        CombinedComponentResult::DidNotFinish,
        CombinedComponentResult::DidNotStart,
        CombinedComponentResult::Disqualified,
    ] {
        let reports = context_reports(status)?;
        assert_clean_winner(&reports)?;
    }
    Ok(())
}

#[test]
fn combined_context_terminal_before_completed_uses_clean_fallback() -> TestResult {
    for status in [
        CombinedComponentResult::DidNotFinish,
        CombinedComponentResult::DidNotStart,
        CombinedComponentResult::Disqualified,
    ] {
        let [complete, terminal, clean] = context_reports(status)?;
        assert_clean_winner(&[terminal, complete, clean])?;
    }
    Ok(())
}
