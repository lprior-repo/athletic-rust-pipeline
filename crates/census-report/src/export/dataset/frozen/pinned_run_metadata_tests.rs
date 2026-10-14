use super::fixtures::TestResult;
use super::token::FrozenInputToken;
use super::{fixtures, objects};
use crate::export::ExportDataset;
use crate::report::ReportError;
use census_domain::model::{
    serialized_bytes, serialized_digest, CensusRun, GradYear, RunManifest, SchoolYear,
};
use census_domain::UsJurisdiction;
use census_store::Store;

mod capture;
mod fenced;
mod retained;
mod token;

pub(super) fn manifest(identity: &str) -> TestResult<RunManifest> {
    let season = SchoolYear::new(2026).ok_or("fixture school year")?;
    Ok(RunManifest {
        store_identity: identity.to_string(),
        run: CensusRun::new(season, 7).ok_or("fixture run")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![
            UsJurisdiction::Wisconsin,
            UsJurisdiction::Illinois,
            UsJurisdiction::Wisconsin,
        ],
    })
}

fn invariant(result: crate::report::ReportResult<FrozenInputToken>) -> TestResult {
    match result {
        Err(ReportError::Invariant { .. }) => Ok(()),
        other => Err(format!("expected construction invariant, got {other:?}").into()),
    }
}
