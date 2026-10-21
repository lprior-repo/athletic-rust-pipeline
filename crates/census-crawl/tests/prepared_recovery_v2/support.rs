use census_crawl::net::{FetchOptions, Fetcher, PreparedRecoveryOutcome};
use census_crawl::{milesplit, AdapterContext, CaptureContext, ResultsCaptureContext};
use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_store::{
    AcquisitionReadBudget, AcquisitionSelectionUsage, CaptureWriterV1, ExactBodySource,
    ExistingRecoveryRoute, PreparedRecoveryBudget, ResultsCaptureOwnerV1, Store,
};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

pub type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub const CAPTURED: &str = "2026-10-01T23:44:16Z";
pub use census_crawl::milesplit::troy_fixture::BODY;

pub struct Fixture {
    pub store: Store,
    writer: CaptureWriterV1,
    pub fetcher: Fetcher,
    _root: tempfile::TempDir,
}

pub struct Prepared {
    pub owner: ResultsCaptureOwnerV1,
    pub work: AcquisitionWorkV1,
    pub target: PreparedRecoveryTarget,
    metadata: Vec<u8>,
    usage: AcquisitionSelectionUsage,
}

impl Fixture {
    pub fn new() -> TestResult<Self> {
        let root = tempfile::tempdir()?;
        Store::create_capture_qualification(root.path().join("store"))?;
        let (store, writer) =
            Store::open_current_qualification_v3(root.path().join("store"))?.into_parts();
        store.bind_run(&RunManifest {
            store_identity: writer.store_identity().as_str().to_owned(),
            run: CensusRun::new(SchoolYear::new(2026).ok_or("year")?, 1).ok_or("run")?,
            cohort: GradYear::CO2027,
            jurisdictions: vec![UsJurisdiction::Michigan],
        })?;
        let fetcher = Fetcher::new(
            store.http_cache_dir(),
            None,
            Duration::ZERO,
            HashMap::new(),
            vec!["mi.milesplit.com".into()],
        )?
        .with_offline(true);
        Ok(Self {
            store,
            writer,
            fetcher,
            _root: root,
        })
    }

    pub fn prepare(&self) -> TestResult<Prepared> {
        let manifest = self.store.run_manifest()?.ok_or("manifest")?;
        let scope = ResultsCaptureScopeV1::new(
            "jurisdiction:MI:2026-27:1",
            UsJurisdiction::Michigan,
            manifest.run,
            AcquisitionDigest::parse(&serialized_digest(&manifest)?)?,
            SelectedHistoryV1::new(HistoryWindow::cohort("2026-10-14")?, 2026)?,
            SourceAdmissionV1::milesplit_results(&"a".repeat(64))?,
            CaptureCacheIntent::ReuseEligible,
        )?;
        let owner = self.writer.results_owner(&self.store, scope)?;
        let reference = reference()?;
        let request =
            milesplit::owned_meet_request(&self.fetcher, &reference, &FetchOptions::default())?;
        let work = owner.for_owned_meet(UsJurisdiction::Michigan, &reference.meet_id, request)?;
        let grant = self.writer.continuation_for_owned_meet(
            &self.store,
            &owner,
            UsJurisdiction::Michigan,
            &reference.meet_id,
        )?;
        let registered = self
            .store
            .register_acquisition_work(&owner, work.clone(), grant)?;
        let reserved = self.store.reserve_acquisition_attempt(registered)?;
        let armed = self.store.arm_acquisition_dispatch(
            reserved,
            DispatchRequestV1::new(
                work.request().original_url(),
                ConditionalRequestV1::Unconditional,
            )?,
        )?;
        let observed = self.store.record_acquisition_response(
            armed.into_observation(),
            ResponseObservationV1::new(200, work.request().original_url(), CAPTURED, vec![])?,
        )?;
        let digest = format!("{:x}", Sha256::digest(BODY));
        let mut metadata = serde_json::json!({
            "url": work.request().original_url(),
            "response_url": work.request().original_url(),
            "method": "GET", "status": 200, "content_digest": digest,
            "bytes": BODY.len(), "fetched_at": CAPTURED,
        });
        metadata.sort_all_objects();
        let metadata = serde_json::to_vec_pretty(&metadata)?;
        let input =
            PreparedCaptureInputV1::new(&digest, u64::try_from(BODY.len())?, metadata.clone())?;
        drop(self.store.prepare_acquisition_capture(
            observed,
            input,
            CaptureDispositionV1::ReusableResponse,
        )?);
        let mut selection = AcquisitionReadBudget::for_selection();
        let ExistingRecoveryRoute::Prepared(target) =
            self.store
                .read_existing_recovery_target(&owner, &work, &mut selection)?
        else {
            return Err("expected a genuine Prepared target".into());
        };
        Ok(Prepared {
            owner,
            work,
            target,
            metadata,
            usage: selection.selection_usage(),
        })
    }

    pub fn publish(&self, prepared: &Prepared) -> TestResult<PathBuf> {
        let digest = format!("{:x}", Sha256::digest(BODY));
        let metadata_digest = format!("{:x}", Sha256::digest(&prepared.metadata));
        let archive = self.store.http_cache_dir().join("archive");
        let body = archive.join("bodies").join(format!("{digest}.body"));
        let metadata = archive
            .join("captures")
            .join(&digest)
            .join(format!("{metadata_digest}.meta.json"));
        std::fs::create_dir_all(body.parent().ok_or("body parent")?)?;
        std::fs::create_dir_all(metadata.parent().ok_or("metadata parent")?)?;
        std::fs::write(&body, BODY)?;
        std::fs::write(metadata, &prepared.metadata)?;
        Ok(body)
    }

    pub fn recover(&self, prepared: &Prepared) -> TestResult<PreparedRecoveryOutcome> {
        let mut budget = PreparedRecoveryBudget::admit(prepared.usage, 1)?;
        Ok(self.fetcher.recover_prepared_target(
            &self.store,
            &prepared.owner,
            &prepared.target,
            &mut budget,
            ExactBodySource::NoneSupplied,
        )?)
    }

    pub fn context<'a>(
        &'a self,
        capture: &'a ResultsCaptureContext,
    ) -> TestResult<AdapterContext<'a>> {
        Ok(AdapterContext {
            capture: CaptureContext::DurableV1(capture),
            fetcher: &self.fetcher,
            store: &self.store,
            refresh: false,
            school_year: SchoolYear::new(2026).ok_or("year")?,
            observed_on: "2026-10-14".into(),
            performance_as_of: chrono::NaiveDate::parse_from_str("2026-10-14", "%Y-%m-%d")?,
            recording: None,
        })
    }
}

pub fn reference() -> TestResult<milesplit::ResultSetRef> {
    milesplit::ResultSetRef::parse("https://mi.milesplit.com/meets/725218/results/1266814/raw")
        .ok_or_else(|| "reference".into())
}
