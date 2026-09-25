use std::sync::Arc;

use census_report::report::ReportResult;
use census_store::Store;
use restate_sdk::prelude::*;

use crate::spawn::Spawner;

use super::{blocking, write_sweep_report, JobError, SweepReport};

/// Persist the sweep report through the blocking pool, updating `report_path` on success.
pub(super) async fn blocking_write_report(
    spawner: Arc<Spawner>,
    store: Arc<Store>,
    report: SweepReport,
    today: String,
) -> Result<Json<SweepReport>, JobError> {
    blocking(spawner, move || -> ReportResult<Json<SweepReport>> {
        let path = write_sweep_report(&store, &report, &today)?;
        let mut report = report;
        report.report_path = Some(path.display().to_string());
        Ok(Json(report))
    })
    .await
}
