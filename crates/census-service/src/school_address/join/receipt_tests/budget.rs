use super::*;
use census_store::{
    ObservationBudget, StoreError, MAX_OBSERVATION_BATCH_BYTES, MAX_OBSERVATION_RECORD_BYTES,
};

#[test]
fn an_oversized_complete_co_op_target_refuses_without_any_effect_witness_or_receipt() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let names: Vec<String> = (0..9)
        .map(|index| format!("Member {index} High School"))
        .collect();
    let main = names.first().ok_or("budget fixture has no main member")?;
    let aliases = names.get(1..).ok_or("budget fixture has no aliases")?;
    let alias_refs: Vec<&str> = aliases.iter().map(String::as_str).collect();
    let mut school = co_op_school(main, "Riverside", &alias_refs)?;
    school.classification = Some("x".repeat(950_000));
    let record_bytes = ObservationBudget::record_bytes(&school)?;
    check!(record_bytes < MAX_OBSERVATION_RECORD_BYTES);
    check!(
        record_bytes
            .checked_mul(names.len())
            .ok_or("budget fixture byte overflow")?
            > MAX_OBSERVATION_BATCH_BYTES
    );
    store.append(Table::Schools, &school)?;
    let entries = names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            entry(
                &format!(
                    "4700000000{:02}",
                    index.checked_add(10).ok_or("member id overflow")?
                ),
                name,
                "Riverside",
                UsJurisdiction::Tennessee,
                &format!("{index} Member Rd"),
            )
        })
        .collect::<TestResult<Vec<_>>>()?;
    let index = DirectoryIndex::build(&entries);
    let (store, before) = physical::checkpoint(store)?;
    let result = process(
        &store,
        &index,
        &lanes(Some("https://nces.ed.gov/ccd.csv"), Some("2026-08-01")),
        Mode::Apply,
    );
    match result {
        Err(JoinError::Store(StoreError::BudgetExceeded {
            what,
            needed,
            available,
        })) => {
            check!(eq; what, "school-address target staged bytes");
            check!(needed > available);
            check!(eq; available, u64::try_from(MAX_OBSERVATION_BATCH_BYTES)?);
        }
        Err(error) => {
            return Err(format!("whole-target budget refusal had wrong reason: {error}").into())
        }
        Ok(_) => return Err("oversized atomic target unexpectedly applied".into()),
    }
    check!(eq; store.scan::<ReviewCase>(Table::ReviewCases)?.len(), 0);
    witness::assert_no_application(&store)?;
    let (_store, after) = physical::checkpoint(store)?;
    check!(eq; after, before);
    check!(eq; after.schools()?.len(), 1);
    Ok(())
}
