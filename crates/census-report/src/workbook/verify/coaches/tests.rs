use super::super::expectations::Expectations;
use super::super::read::tests::{
    alter_sheet, copy_bundle, publication, recapture, rejected_by_all, replace_cell, TestResult,
};

#[test]
fn displaced_coach_ids_do_not_bypass_any_observation_cell() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (good, dataset, options) = publication(directory.path())?;
    let expectations = Expectations::of(&dataset, &options)?;
    let ordered = super::ordered(&expectations);
    check!(eq; ordered.len(), 2, "one claim for Alpha's two observations and one for Beta");
    let first = ordered.first().ok_or("first coach")?.0.id.as_str();
    let last = ordered.last().ok_or("last coach")?.0.id.as_str();
    check!(first != last, "the swap needs distinct coach identities");
    for (cell, column, value) in [
        ("G2", 6, last),
        ("A2", 0, "foreign-school"),
        ("J2", 9, "forged@fixture.test"),
        ("I2", 8, "forged-role"),
        ("Q2", 16, "current_declared"),
        ("O2", 14, "https://foreign.test/staff"),
        ("P2", 15, "1900-01-01"),
    ] {
        let path = copy_bundle(&good, &directory.path().join(cell))?;
        alter_sheet(&path, "Coaches", |range| {
            replace_cell(range, (1, 6), last);
            replace_cell(range, (2, 6), first);
            replace_cell(range, (1, column), value);
            Ok(())
        })?;
        recapture(&path, &dataset, &options)?;
        rejected_by_all(&path, &dataset, &options, &format!("Coaches!{cell}"))?;
    }
    Ok(())
}
