use super::*;
use crate::restate_services::jurisdiction::capture_admission;
use crate::restate_services::plan::{compute_plan_fingerprint, BrowserLaneState};
use crate::restate_services::wire::{JurisdictionRequest, JurisdictionState, SourcePlan};
use crate::restate_services::{CaptureExecution, Jobs};
use census_domain::model::Revision;
use census_domain::model::{
    CensusRun, GradYear, HistoryWindow, RunManifest, SchoolYear, SourceMeetRef,
};
use census_domain::UsJurisdiction;
use census_reconcile::identity::WorkflowIdentity;
use census_store::{clock::SystemClock, Store, Table};
use std::collections::HashMap;

#[path = "v2/continuation_fixture.rs"]
mod continuation;

pub(super) const RESULTS_URL: &str = "https://mi.milesplit.com/meets/725218/results";
pub(super) const RAW_URL: &str = "https://mi.milesplit.com/meets/725218/results/1266814/raw";
const MEET_PAGE: &[u8] =
    br#"<script>meetResultFiles = [{"id":1266814,"name":"Results","isMeetPro":0}];</script>"#;

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    pub(super) store: Arc<Store>,
    pub(super) fetcher: Arc<Fetcher>,
    pub(super) region: Arc<crate::spawn::Spawner>,
    pub(super) jobs: Jobs,
    pub(super) request: JurisdictionRequest,
    pub(super) state: JurisdictionState,
    pub(super) bridge: ResultsStageBridge,
}

#[derive(Clone)]
pub(super) struct ResultsStageBridge {
    owner: JurisdictionCensus,
}

#[object]
impl ResultsStageBridge {
    #[handler]
    async fn run(
        &self,
        ctx: ObjectContext<'_>,
        Json(request): Json<JurisdictionRequest>,
    ) -> Result<Json<ResultsStageOutcome>, HandlerError> {
        let mut state = self.owner.load_object(&ctx).await?;
        self.owner
            .results_owed(&ctx, &request, &mut state, "2026-10-14")
            .await?;
        state
            .history
            .results
            .remove(&2026)
            .map(Json)
            .ok_or_else(|| jobs::invariant("Results fixture year was not returned"))
    }
}

impl Fixture {
    pub(super) fn new() -> TestResult<Self> {
        Self::with_protocol(census_domain::model::CaptureProtocolVersion::V1)
    }

    pub(super) fn v2() -> TestResult<Self> {
        Self::with_protocol(census_domain::model::CaptureProtocolVersion::V2)
    }

    fn with_protocol(protocol: census_domain::model::CaptureProtocolVersion) -> TestResult<Self> {
        Self::with_inputs(protocol, request()?, meet())
    }

    pub(super) fn with_inputs(
        protocol: census_domain::model::CaptureProtocolVersion,
        request: JurisdictionRequest,
        meet: SourceMeetRef,
    ) -> TestResult<Self> {
        let root = tempfile::tempdir()?;
        let (store, writer) = open(root.path(), protocol)?;
        let store = Arc::new(store);
        store.bind_run(&RunManifest {
            store_identity: writer.store_identity().as_str().to_owned(),
            run: CensusRun::new(request.season, request.revision.get()).ok_or("fixture run")?,
            cohort: GradYear::CO2027,
            jurisdictions: vec![request.jurisdiction],
        })?;
        let state = state(&request);
        store.append(Table::SourceMeets, &meet)?;
        let fetcher = Arc::new(
            Fetcher::new(
                store.http_cache_dir(),
                None,
                Duration::ZERO,
                HashMap::new(),
                request.authorized_hosts.clone(),
            )?
            .with_offline(true),
        );
        crate::capture_cache::seed(
            fetcher.cache_dir(),
            &meet.results_url,
            MEET_PAGE,
            "2026-10-01T23:44:16Z",
            &[],
        )?;
        let region = Arc::new(crate::spawn::Spawner::with_capacity(1));
        let jobs = Jobs::new(Arc::clone(&store), Arc::clone(&region));
        let owner = JurisdictionCensus::new(
            Arc::clone(&store),
            Arc::new(SystemClock),
            None,
            CaptureExecution::FreshQualification(Arc::new(writer)),
            jobs.clone(),
        );
        *owner.fetcher.try_lock()? =
            Some((request.authorized_hosts.clone(), 1, Arc::clone(&fetcher)));
        let bridge = ResultsStageBridge { owner };
        Ok(Self {
            _root: root,
            store,
            fetcher,
            region,
            jobs,
            request,
            state,
            bridge,
        })
    }

    pub(super) fn key(&self) -> &str {
        &self.state.identity
    }

    pub(super) fn capture(&self) -> TestResult<capture_admission::ResultsCaptureExecution> {
        let sources = SourceSelection::parse(&["milesplit".to_owned()])
            .map_err(crate::restate_services::tests::sdk_error)?;
        Ok(capture_admission::admit_results_owner(
            &self.store,
            &self.bridge.owner.capture,
            self.key(),
            &self.request,
            &self.state,
            (2026, sources),
        )?)
    }

    pub(super) fn seed_failed_attempt(&self) -> TestResult {
        use census_domain::model::{AcquisitionFailureV1, RequestRepresentationV1};
        let sources = SourceSelection::parse(&["milesplit".to_owned()])
            .map_err(crate::restate_services::tests::sdk_error)?;
        let capture = capture_admission::admit_results_owner(
            &self.store,
            &self.bridge.owner.capture,
            self.key(),
            &self.request,
            &self.state,
            (2026, sources),
        )?;
        let capture_admission::ResultsCaptureExecution::FreshQualification {
            writer, owner, ..
        } = capture
        else {
            return Err("fixture original owner was not admitted".into());
        };
        let work = owner.for_owned_meet(
            UsJurisdiction::Michigan,
            "725218",
            RequestRepresentationV1::get(
                &owned_url(),
                Vec::new(),
                census_crawl::net::DEFAULT_USER_AGENT,
            )?,
        )?;
        let grant = writer.continuation_for_owned_meet(
            &self.store,
            &owner,
            UsJurisdiction::Michigan,
            "725218",
        )?;
        let work = self.store.register_acquisition_work(&owner, work, grant)?;
        let attempt = self.store.reserve_acquisition_attempt(work)?;
        self.store
            .refuse_acquisition_attempt(attempt, AcquisitionFailureV1::AdmissionRefused)?;
        Ok(())
    }
}

pub(super) fn owned_url() -> String {
    format!(
        "https://mi.milesplit.com/api/v1/meets/725218/performances?isMeetPro=0&fields={}",
        census_crawl::milesplit::OWNED_FIELDS
    )
}

fn request() -> TestResult<JurisdictionRequest> {
    Ok(JurisdictionRequest {
        jurisdiction: UsJurisdiction::Michigan,
        season: SchoolYear::new(2026).ok_or("fixture season")?,
        revision: Revision(1),
        history: HistoryWindow::new(2026, 2026, "2026-10-14")?,
        refresh: false,
        limit_per_state: None,
        concurrency: 1,
        observed_on: None,
        authorized_hosts: vec!["mi.milesplit.com".to_owned()],
        source_parallelism: 1,
    })
}

fn state(request: &JurisdictionRequest) -> JurisdictionState {
    JurisdictionState {
        identity: WorkflowIdentity::jurisdiction(
            request.jurisdiction,
            request.season,
            request.revision,
        )
        .as_str()
        .to_owned(),
        history_window: Some(request.history),
        plan: Some(SourcePlan {
            sweepable: vec!["milesplit".to_owned()],
            refused: Vec::new(),
            fingerprint: compute_plan_fingerprint(
                request.jurisdiction,
                request.season,
                request.revision,
                BrowserLaneState::Absent,
            ),
        }),
        ..JurisdictionState::default()
    }
}

pub(super) fn meet() -> SourceMeetRef {
    SourceMeetRef {
        id: SourceMeetRef::row_id("milesplit", "725218"),
        source: "milesplit".to_owned(),
        source_meet_id: "725218".to_owned(),
        jurisdiction: UsJurisdiction::Michigan,
        season: "xc".to_owned(),
        year: 2026,
        name: "Results replay fixture".to_owned(),
        date: Some("2026-09-28".to_owned()),
        venue: "Published fixture venue".to_owned(),
        results_url: RESULTS_URL.to_owned(),
        observed_on: "2026-10-01".to_owned(),
    }
}

fn open(
    root: &std::path::Path,
    protocol: census_domain::model::CaptureProtocolVersion,
) -> TestResult<(Store, census_store::CaptureWriterV1)> {
    use census_domain::model::CaptureProtocolVersion;
    let path = root.join("store");
    Ok(match protocol {
        CaptureProtocolVersion::V1 => {
            Store::create_capture_qualification_v1(&path)?;
            Store::open_capture_qualification_v1(path)?.into_parts()
        }
        CaptureProtocolVersion::V2 => {
            Store::create_capture_qualification_v2(&path)?;
            Store::open_capture_qualification_v2(path)?.into_parts()
        }
    })
}
