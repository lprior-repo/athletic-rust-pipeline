use super::retention::files;
use super::*;

mod authentication;
mod cases;
mod provenance;

#[test]
fn native_independent_reviewed_roster_producer_replays_all_cases_across_changed_build() -> TestResult
{
    let retained = authentication::read()?;
    provenance::different_current_build(&retained)?;
    for case in retained
        .manifest
        .get("cases")
        .and_then(Value::as_array)
        .ok_or("retained cases")?
    {
        cases::replay(&retained, case)?;
    }
    check!(eq; serde_json::to_value(files::inventory(&retained.root)?)?, retained.inventory);
    check!(eq; files::read(&retained.path)?, retained.bytes);
    provenance::current_qualification()?;
    check!(eq; serde_json::to_value(files::inventory(&retained.root)?)?, retained.inventory);
    check!(eq; files::read(&retained.path)?, retained.bytes);
    Ok(())
}
