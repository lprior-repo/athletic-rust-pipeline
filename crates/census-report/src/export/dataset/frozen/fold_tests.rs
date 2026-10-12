use super::super::fixtures::{school, seeded_store, TestResult};
use super::super::{freeze_v2, ReportError};
use census_store::Table;

#[test]
fn freeze_fold_refuses_giant_entity_before_accepting_its_window() -> TestResult {
    let seeded = seeded_store()?;
    let base = school("Giant Fold High");
    for index in 0..12_u32 {
        let mut row = base.clone();
        row.aliases
            .push(format!("{index:02}{}", "x".repeat(100_000)));
        seeded.store.append(Table::Schools, &row)?;
    }
    let error = match freeze_v2(&seeded.store, "freeze-fold:giant") {
        Ok(_) => return Err("giant entity was accepted".into()),
        Err(error) => error,
    };
    check!(matches!(
        error,
        ReportError::Store(census_store::StoreError::BudgetExceeded {
            what: "canonical entity encoded fold",
            ..
        })
    ));
    check!(eq; seeded.store.snapshot().scan::<census_domain::model::CanonicalSchool>(Table::Schools)?.len(), 2);
    Ok(())
}
