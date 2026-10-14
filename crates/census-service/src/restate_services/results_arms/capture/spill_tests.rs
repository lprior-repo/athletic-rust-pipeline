use super::*;
use crate::restate_services::results_arms::capture_tests::{meet, scope};
use crate::restate_services::results_arms::{budget::Admission, selection};

#[path = "spill_tests/integrity_tests.rs"]
mod integrity_tests;
#[path = "spill_tests/lifecycle_tests.rs"]
mod lifecycle_tests;
#[path = "spill_tests/parity_tests.rs"]
mod parity_tests;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn uncancelled() -> AtomicBool {
    AtomicBool::new(false)
}

pub(super) fn build(
    root: &Path,
    rows: Vec<SourceMeetRef>,
) -> StoreResult<(Arc<Dataset>, Arc<SpillAdmission>)> {
    let admission = Arc::new(SpillAdmission::default());
    let mut build = Build::new(root, Arc::clone(&admission))?;
    rows.into_iter()
        .try_for_each(|row| build.push(row, &uncancelled()))?;
    Ok((Arc::new(build.finish(&uncancelled())?), admission))
}

pub(super) fn collect(dataset: &Dataset) -> StoreResult<Vec<SourceMeetRef>> {
    let mut rows = Vec::new();
    dataset.visit(UsJurisdiction::Wisconsin, &uncancelled(), |row, _charge| {
        rows.push(row.clone());
        Ok(())
    })?;
    Ok(rows)
}

pub(super) fn selected(
    dataset: Arc<Dataset>,
    admission: Arc<Admission>,
) -> StoreResult<selection::Selected> {
    let scope =
        scope(UsJurisdiction::Wisconsin).map_err(|error| owner::refused(&error.to_string()))?;
    selection::partition(scope, admission, dataset, &uncancelled()).map(|(selected, _)| selected)
}
