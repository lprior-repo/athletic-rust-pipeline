use super::*;

fn assert_historical_selection(policy: u32) -> TestResult {
    for status in [
        CombinedComponentResult::DidNotFinish,
        CombinedComponentResult::DidNotStart,
        CombinedComponentResult::Disqualified,
    ] {
        let [complete, terminal, clean] = context_reports(status)?;
        let representative = if complete.performance.id < terminal.performance.id {
            &complete.performance
        } else {
            &terminal.performance
        };
        for order in [
            vec![complete.clone(), terminal.clone(), clean.clone()],
            vec![terminal.clone(), complete.clone(), clean.clone()],
        ] {
            assert_selection(&order, policy, representative, 2)?;
        }
        for raw in ["DNF", "DNS", "DQ"] {
            let mut raw_terminal = terminal.clone();
            raw_terminal.performance.mark = Mark::Raw(raw.into());
            raw_terminal.performance.combined_completion = None;
            for order in [
                vec![complete.clone(), raw_terminal.clone(), clean.clone()],
                vec![raw_terminal.clone(), complete.clone(), clean.clone()],
            ] {
                assert_selection(&order, policy, &complete.performance, 2)?;
            }
        }
    }
    Ok(())
}

#[test]
fn combined_context_policy_eighteen_keeps_numeric_completion_and_raw_status_meaning() -> TestResult
{
    assert_historical_selection(18)
}

#[test]
fn combined_context_policy_nineteen_keeps_numeric_completion_and_raw_status_meaning() -> TestResult
{
    assert_historical_selection(19)
}
