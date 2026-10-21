use super::{ExportDataset, Fixture, Table, TestResult};
use crate::export::dataset::{
    decoded_generations, generation_counter_window, resident_generations,
};
use crate::report::ReportError;
use census_domain::model::{CanonicalMeet, CompetitionLevel};
use census_store::StoreError;

#[tokio::test]
async fn selected_capture_refuses_oversized_input_before_generation_and_proof_objects() -> TestResult
{
    let fixture = Fixture::new().await?;
    let objects = super::temporary()?;
    let mut row = CanonicalMeet::new_checked(
        None,
        "Selected preflight oversized input",
        "2026-09-01",
        CompetitionLevel::Dual,
    )?;
    row.location = Some("x".repeat(1024 * 1024));
    let bytes = serde_json::to_vec(&row)?.len();
    fixture.store.append(Table::Meets, &row)?;
    let _counter_window = generation_counter_window()?;
    let decoded = decoded_generations();
    let resident = resident_generations();
    let result = ExportDataset::from_selected_snapshot(
        &fixture.store,
        &fixture.store.snapshot(),
        &fixture.selection,
        objects.path(),
    );
    check!(matches!(result,
        Err(ReportError::Store(StoreError::RowTooLarge { bytes: actual, max }))
            if actual == bytes && max == 1024 * 1024));
    check!(eq; decoded_generations(), decoded);
    check!(eq; resident_generations(), resident);
    check!(std::fs::read_dir(objects.path())?.next().is_none());
    Ok(())
}

#[tokio::test]
async fn selected_preflight_retains_pinned_snapshot_semantics_after_live_growth() -> TestResult {
    let fixture = Fixture::new().await?;
    let first_objects = super::temporary()?;
    let first = fixture.capture(first_objects.path())?;
    let expected = super::witness::capture(&first)?;
    let snapshot = fixture.store.snapshot();
    let mut oversized = CanonicalMeet::new_checked(
        None,
        "Later oversized input",
        "2026-09-01",
        CompetitionLevel::Dual,
    )?;
    oversized.location = Some("x".repeat(1024 * 1024));
    fixture.store.append(Table::Meets, &oversized)?;
    let pinned_objects = super::temporary()?;
    let pinned = ExportDataset::from_selected_snapshot(
        &fixture.store,
        &snapshot,
        &fixture.selection,
        pinned_objects.path(),
    )?;
    check!(eq; super::witness::capture(&pinned)?, expected);
    let current_objects = super::temporary()?;
    check!(matches!(
        ExportDataset::from_selected_snapshot(
            &fixture.store,
            &fixture.store.snapshot(),
            &fixture.selection,
            current_objects.path(),
        ),
        Err(ReportError::Store(StoreError::RowTooLarge { .. }))
    ));
    check!(std::fs::read_dir(current_objects.path())?.next().is_none());
    Ok(())
}
