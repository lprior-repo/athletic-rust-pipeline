use census_crawl::net::cache::{seed_eligible_occurrence, EligibleSeed};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::{milesplit, ResultsCaptureContext};
use census_domain::model::{
    serialized_digest, AcquisitionDigest, CaptureCacheIntent, CensusRun, GradYear, HistoryWindow,
    ResultsCaptureScopeV1, RunManifest, SchoolYear, SelectedHistoryV1, SourceAdmissionV1,
};
use census_domain::UsJurisdiction;
use census_store::{CaptureWriterV1, ResultsCaptureOwnerV1, Store};

pub type TestResult<T = ()> = Result<T, anyhow::Error>;

pub struct DurableCapture {
    _root: Option<tempfile::TempDir>,
    store: Store,
    writer: CaptureWriterV1,
    owner: ResultsCaptureOwnerV1,
}

impl DurableCapture {
    pub fn mint() -> TestResult<Self> {
        let root = tempfile::tempdir()?;
        Store::create_capture_qualification_v2(root.path().join("store"))?;
        let (store, writer) =
            Store::open_capture_qualification_v2(root.path().join("store"))?.into_parts();
        let capture = Self::adopt(store, writer)?;
        Ok(Self {
            _root: Some(root),
            ..capture
        })
    }

    pub fn adopt(store: Store, writer: CaptureWriterV1) -> TestResult<Self> {
        store.bind_run(&RunManifest {
            store_identity: writer.store_identity().as_str().to_owned(),
            run: CensusRun::new(
                SchoolYear::new(2026).ok_or_else(|| anyhow::anyhow!("2026 school year"))?,
                1,
            )
            .ok_or_else(|| anyhow::anyhow!("census run"))?,
            cohort: GradYear::CO2027,
            jurisdictions: vec![UsJurisdiction::Alabama],
        })?;
        let manifest = store
            .run_manifest()?
            .ok_or_else(|| anyhow::anyhow!("capture run manifest"))?;
        let scope = ResultsCaptureScopeV1::new(
            "jurisdiction:AL:2026-27:1",
            UsJurisdiction::Alabama,
            manifest.run,
            AcquisitionDigest::parse(&serialized_digest(&manifest)?)?,
            SelectedHistoryV1::new(HistoryWindow::cohort("2026-10-14")?, 2026)?,
            SourceAdmissionV1::milesplit_results(&"a".repeat(64))?,
            CaptureCacheIntent::ReuseEligible,
        )?;
        let owner = writer.results_owner(&store, scope)?;
        Ok(Self {
            _root: None,
            store,
            writer,
            owner,
        })
    }

    #[allow(dead_code)]
    pub fn store(&self) -> &Store {
        &self.store
    }

    pub fn capture(&self) -> TestResult<ResultsCaptureContext> {
        let grant = self.writer.continuation_for_owned_meet(
            &self.store,
            &self.owner,
            UsJurisdiction::Alabama,
            "725218",
        )?;
        Ok(ResultsCaptureContext::new(
            self.owner.clone(),
            grant,
            UsJurisdiction::Alabama,
            "725218",
        )?)
    }

    #[allow(dead_code)]
    pub fn record_owned_meet(
        &self,
        fetcher: &Fetcher,
        reference: &milesplit::ResultSetRef,
        body: &[u8],
        fetched_at: &str,
    ) -> TestResult<()> {
        let request = milesplit::owned_meet_request(fetcher, reference, &FetchOptions::default())?;
        let work = self
            .owner
            .for_owned_meet(UsJurisdiction::Alabama, "725218", request)?;
        let grant = self.writer.continuation_for_owned_meet(
            &self.store,
            &self.owner,
            UsJurisdiction::Alabama,
            "725218",
        )?;
        seed_eligible_occurrence(EligibleSeed {
            store: &self.store,
            owner: &self.owner,
            grant,
            work,
            fetcher,
            body,
            fetched_at,
        })?;
        Ok(())
    }
}
