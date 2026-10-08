use super::{oracle, TestResult};
use census_store::Table;

fn rejection(result: Result<(), String>, label: &str) -> TestResult<String> {
    match result {
        Ok(()) => Err(format!("the kill barrier accepted {label}").into()),
        Err(reason) => Ok(reason),
    }
}

#[test]
fn e_kill_barrier_rejects_zero_and_completed_merge_progress() -> TestResult {
    let all = Table::ALL.len();
    let zero = rejection(
        oracle::require_mid_merge_progress(0, all),
        "zero reached progress",
    )?;
    check!(
        zero.contains("never reached its merge phase"),
        "the refusal must name the unreached merge phase: {zero}"
    );
    let complete = rejection(
        oracle::require_mid_merge_progress(all, all),
        "a fully written merge",
    )?;
    check!(
        complete.contains("never had to resume"),
        "the refusal must name the unused resume: {complete}"
    );
    check!(
        oracle::require_mid_merge_progress(1, all).is_ok(),
        "a mid-merge barrier must accept a partial merge"
    );
    Ok(())
}
