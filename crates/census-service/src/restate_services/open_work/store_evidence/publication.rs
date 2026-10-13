use census_domain::model::{CensusRun, GradYear};
use census_report::export::store_lineage;
use census_report::report::Scope;
use census_report::workbook::publication::{current_workbook, open_publication};
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
    let lineage = store_lineage(store)?;
    if lineage.run != Some(run) {
        return Ok(false);
    }
    match open_publication(&path, &lineage, Scope::Core, GradYear::CO2027) {
        Ok(_) => {}
        Err(core_error) => {
            match open_publication(&path, &lineage, Scope::AllSources, GradYear::CO2027) {
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(%core_error, %error, "publication obligation failed exact verification");
                    return Ok(false);
                }
            }
        }
    }
    let current = store_lineage(store)?;
    Ok(current.source_digest == lineage.source_digest
        && current.run == lineage.run
        && current.cohort == lineage.cohort)
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_domain::model::{RunManifest, SchoolYear};
    use census_domain::UsJurisdiction;
    use census_report::export::{decoded_generations, resident_generations, store_identity};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn a_publication_obligation_verifies_with_one_decoded_generation() -> TestResult {
        let temporary = tempfile::tempdir()?;
        let store = Store::open(temporary.path().join("store"))?;
        let school_year = SchoolYear::new(2026).ok_or("fixture school year")?;
        let run = CensusRun::new(school_year, 1).ok_or("fixture census run")?;
        store.bind_run(&RunManifest {
            store_identity: store_identity(&store)?,
            run,
            cohort: GradYear::CO2027,
            jurisdictions: vec![UsJurisdiction::Wisconsin],
        })?;
        census_report::workbook::build(
            &store,
            &census_report::workbook::Options {
                grad_year: Some(2027),
                out: None,
                limit: None,
                scope: Scope::AllSources,
                jurisdictions: Vec::new(),
                school_year,
            },
        )?;
        let decoded = decoded_generations();
        let resident = resident_generations();
        check!(eq; inspect(&store, run), Disposition::Complete);
        check!(eq; decoded_generations(), decoded.checked_add(1).ok_or("decode counter overflow")?);
        check!(eq; resident_generations(), resident);
        Ok(())
    }
}
