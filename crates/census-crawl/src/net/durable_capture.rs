use crate::context::ResultsCaptureContext;
use crate::{CrawlError, CrawlResult};
use census_domain::model::{
    serialized_digest, AcquisitionDigest, CaptureCacheIntent, CensusRun, GradYear, HistoryWindow,
    ResultsCaptureScopeV1, RunManifest, SchoolYear, SelectedHistoryV1, SourceAdmissionV1,
};
use census_domain::UsJurisdiction;
use census_store::Store;

pub fn durable_capture_context(root: &std::path::Path) -> CrawlResult<ResultsCaptureContext> {
    Store::create_capture_qualification_v1(root)?;
    let (store, writer) = Store::open_capture_qualification_v1(root)?.into_parts();
    let manifest = RunManifest {
        store_identity: writer.store_identity().as_str().to_owned(),
        run: CensusRun::new(
            SchoolYear::new(2026).ok_or(CrawlError::Invariant {
                detail: "durable capture harness requires the 2026 school year".into(),
            })?,
            1,
        )
        .ok_or(CrawlError::Invariant {
            detail: "durable capture harness requires run revision 1".into(),
        })?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![UsJurisdiction::Michigan],
    };
    store.bind_run(&manifest)?;
    let scope = ResultsCaptureScopeV1::new(
        "jurisdiction:MI:2026-27:1",
        UsJurisdiction::Michigan,
        manifest.run,
        AcquisitionDigest::parse(&serialized_digest(&manifest).map_err(|error| {
            CrawlError::Invariant {
                detail: error.to_string(),
            }
        })?)
        .map_err(|error| CrawlError::Invariant {
            detail: error.to_string(),
        })?,
        SelectedHistoryV1::new(
            HistoryWindow::cohort("2026-10-14").map_err(|error| CrawlError::Invariant {
                detail: error.to_string(),
            })?,
            2026,
        )
        .map_err(|error| CrawlError::Invariant {
            detail: error.to_string(),
        })?,
        SourceAdmissionV1::milesplit_results(&"a".repeat(64)).map_err(|error| {
            CrawlError::Invariant {
                detail: error.to_string(),
            }
        })?,
        CaptureCacheIntent::ReuseEligible,
    )
    .map_err(|error| CrawlError::Invariant {
        detail: error.to_string(),
    })?;
    let owner = writer.results_owner(&store, scope)?;
    let grant =
        writer.continuation_for_owned_meet(&store, &owner, UsJurisdiction::Michigan, "725218")?;
    ResultsCaptureContext::new(owner, grant, UsJurisdiction::Michigan, "725218")
}
