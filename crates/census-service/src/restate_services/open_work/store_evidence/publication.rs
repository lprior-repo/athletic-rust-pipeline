use census_domain::model::{CensusRun, GradYear};
use census_report::export::ExportDataset;
use census_report::report::Scope;
use census_report::workbook::publication::{current_workbook, verify_for_seal};
use census_store::Store;

use super::Disposition;

pub(super) fn inspect(store: &Store, run: CensusRun) -> Disposition {
    match verify(store, run) {
        Ok(true) => Disposition::Complete,
        Ok(false) => Disposition::Partial,
        Err(error) => {
            tracing::warn!(%error, "publication obligation is unreadable or absent");
            Disposition::Unknown
        }
    }
}

fn verify(store: &Store, run: CensusRun) -> census_report::report::ReportResult<bool> {
    let path = current_workbook(&store.out_dir().join("publication"))?;
    let dataset = ExportDataset::load(store)?;
    if dataset.lineage.run != Some(run) {
        return Ok(false);
    }
    match verify_for_seal(&path, &dataset, Scope::Core, GradYear::CO2027) {
        Ok(_) => {}
        Err(core_error) => {
            match verify_for_seal(&path, &dataset, Scope::AllSources, GradYear::CO2027) {
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(%core_error, %error, "publication obligation failed exact verification");
                    return Ok(false);
                }
            }
        }
    }
    dataset.ensure_current(store)?;
    Ok(true)
}
