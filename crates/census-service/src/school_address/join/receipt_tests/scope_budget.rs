use super::*;
use census_store::{StoreError, MAX_OBSERVATION_BATCH_BYTES};

fn oversized_scope() -> LaneSet {
    let mut scope = lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01"));
    let (_, unused) = lane_of(
        "unused",
        &"x".repeat(MAX_OBSERVATION_BATCH_BYTES),
        None,
        None,
        Vec::new(),
    );
    scope.by_provider.insert("unused".to_string(), vec![unused]);
    scope
}

fn matching_index() -> TestResult<DirectoryIndex> {
    Ok(DirectoryIndex::build(&[entry(
        "390000000001",
        "Springfield High School",
        "Springfield",
        UsJurisdiction::Ohio,
        "123 Main St",
    )?]))
}

#[test]
fn oversized_application_scope_refuses_before_any_persisted_change() -> TestResult {
    let (_dir, store) = store_with_school()?;
    let (store, before) = physical::checkpoint(store)?;
    let result = process(&store, &matching_index()?, &oversized_scope(), Mode::Apply);
    match result {
        Err(JoinError::Store(StoreError::BudgetExceeded {
            what,
            needed,
            available,
        })) => {
            check!(eq; what, "school-address application scope bytes");
            check!(needed > available);
            check!(eq; available, u64::try_from(MAX_OBSERVATION_BATCH_BYTES)?);
        }
        Err(error) => return Err(format!("wrong scope refusal: {error}").into()),
        Ok(_) => return Err("oversized application scope was admitted".into()),
    }
    witness::assert_no_application(&store)?;
    let (_store, after) = physical::checkpoint(store)?;
    check!(eq; before, after);
    Ok(())
}

#[test]
fn dry_run_skips_application_scope_admission_and_preserves_all_bytes() -> TestResult {
    let (_dir, store) = store_with_school()?;
    let (store, before) = physical::checkpoint(store)?;
    let (counters, outcomes) =
        process(&store, &matching_index()?, &oversized_scope(), Mode::DryRun)?;
    check!(eq; counters.scanned, 1);
    check!(eq; counters.linked, 1);
    check!(eq; outcomes.len(), 0);
    witness::assert_no_application(&store)?;
    let (_store, after) = physical::checkpoint(store)?;
    check!(eq; before, after);
    Ok(())
}
