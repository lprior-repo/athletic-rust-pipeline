use super::*;
use crate::restate_services::{
    history_stage::HistoricalStageScope, source_selection::SourceSelection,
};
use census_crawl::net::Fetcher;
use census_domain::model::{
    serialized_digest, AcquisitionDigest, AcquisitionWorkId, CaptureCacheIntent, CensusRun,
    GradYear, HistoryWindow, ResultsCaptureScopeV1, RunManifest, SchoolYear, SelectedHistoryV1,
    SourceAdmissionV1, SourceMeetRef,
};
use census_domain::UsJurisdiction;
use census_store::{AcquisitionReadBudget, ResultsCaptureOwnerV1, Store};
use std::{collections::HashMap, sync::Arc, time::Duration};

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    store: Arc<Store>,
    fetcher: Arc<Fetcher>,
    owner: ResultsCaptureOwnerV1,
    scope: HistoricalStageScope,
}

impl Fixture {
    pub(super) fn new() -> TestResult<Self> {
        let root = tempfile::tempdir()?;
        let path = root.path().join("store");
        Store::create_capture_qualification(&path)?;
        let (store, writer) = Store::open_current_qualification_v3(&path)?.into_parts();
        let store = Arc::new(store);
        let run = CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("fixture run")?;
        let manifest = RunManifest {
            store_identity: writer.store_identity().as_str().to_owned(),
            run,
            cohort: GradYear::CO2027,
            jurisdictions: vec![UsJurisdiction::Michigan],
        };
        store.bind_run(&manifest)?;
        let history = HistoryWindow::new(2026, 2026, "2026-10-14")?;
        let scope = ResultsCaptureScopeV1::new(
            "jurisdiction:MI:2026-27:1",
            UsJurisdiction::Michigan,
            run,
            AcquisitionDigest::parse(&serialized_digest(&manifest)?)?,
            SelectedHistoryV1::new(history, 2026)?,
            SourceAdmissionV1::milesplit_results(&"a".repeat(64))?,
            CaptureCacheIntent::ReuseEligible,
        )?;
        let owner = writer.results_owner(&store, scope)?;
        let fetcher = Arc::new(fetcher(&store, "fixture-agent")?);
        let scope = HistoricalStageScope {
            jurisdiction: UsJurisdiction::Michigan,
            year: 2026,
            refresh: false,
            window: history,
            observed_on: chrono::NaiveDate::from_ymd_opt(2026, 10, 14).ok_or("fixture date")?,
        };
        Ok(Self {
            _root: root,
            store,
            fetcher,
            owner,
            scope,
        })
    }

    pub(super) fn plan(&self) -> TestResult<ResultsPlanV2> {
        let meet = meet();
        let reference = census_crawl::milesplit::ResultSetRef::parse(&meet.results_url)
            .ok_or("fixture reference")?;
        let request = census_crawl::milesplit::owned_meet_request(
            &self.fetcher,
            &reference,
            &Default::default(),
        )?;
        let work = self
            .owner
            .for_owned_meet(UsJurisdiction::Michigan, "123", request)?;
        let mut usage = AcquisitionReadBudget::for_selection();
        let route = self
            .store
            .read_existing_recovery_target(&self.owner, &work, &mut usage)?;
        let binding = ResultsPlanBinding::new(
            self.owner.store_identity().clone(),
            self.owner.scope().clone(),
            "2026-10-14".to_owned(),
        )?;
        let mut builder = PlanBuilder::new(
            binding,
            vec!["milesplit".to_owned()],
            vec![meet],
            Vec::new(),
        )?;
        let target = builder.record_target(work.slot().id()?, work.id()?, route)?;
        builder.record_meet(
            Discovery::Failed(DiscoveryFailure {
                category: DiscoveryFailureCategory::Fetch,
                detail: "fixture discovery unavailable".to_owned(),
            }),
            MeetRoute::Owned(target),
        )?;
        Ok(builder.finish(SelectionUsage::observed(&usage.selection_usage())?)?)
    }

    fn require(
        &self,
        plan: &mut ResultsPlanV2,
        fetcher: &Arc<Fetcher>,
    ) -> Result<(), restate_sdk::prelude::HandlerError> {
        plan.require_execution_binding(
            &self.store,
            fetcher,
            &self.owner,
            self.scope,
            SourceSelection::parse(&["milesplit".to_owned()])?,
        )
    }
}

fn fetcher(store: &Store, agent: &str) -> TestResult<Fetcher> {
    Ok(Fetcher::new(
        store.http_cache_dir(),
        Some(agent.to_owned()),
        Duration::ZERO,
        HashMap::new(),
        vec!["mi.milesplit.com".to_owned()],
    )?
    .with_offline(true))
}

fn meet() -> SourceMeetRef {
    SourceMeetRef {
        id: "milesplit:123".to_owned(),
        source: "milesplit".to_owned(),
        source_meet_id: "123".to_owned(),
        jurisdiction: UsJurisdiction::Michigan,
        season: "xc".to_owned(),
        year: 2026,
        name: "fixture".to_owned(),
        date: Some("2026-09-01".to_owned()),
        venue: String::new(),
        results_url: "https://mi.milesplit.com/meets/123/results".to_owned(),
        observed_on: "2026-10-14".to_owned(),
    }
}

#[test]
fn sdk_replay_rebinds_the_actual_structured_work_before_effects() -> TestResult {
    let fixture = Fixture::new()?;
    let mut bytes = encode(&fixture.plan()?)?;
    fixture
        .require(&mut decode(&mut bytes)?, &fixture.fetcher)
        .map_err(crate::restate_services::tests::sdk_error)?;
    Ok(())
}

#[test]
fn sdk_large_ordinary_plan_rebinds_and_reserves_before_collection() -> TestResult {
    let fixture = Fixture::new()?;
    let mut plan = fixture.plan()?;
    let meet = plan.meets.first().ok_or("fixture meet")?.clone();
    let route = *plan.routes.first().ok_or("fixture route")?;
    let options = census_crawl::milesplit::ResultSetOptions {
        urls: (100_001..=100_002)
            .map(|id| census_crawl::milesplit::ResultSetRequest {
                url: format!("https://mi.milesplit.com/meets/123/results/{id}/raw"),
                jurisdiction: UsJurisdiction::Michigan,
            })
            .collect(),
    };
    plan.meets = vec![meet; 2048];
    plan.routes = vec![route; 2048];
    plan.discovery = (0..2048)
        .map(|_| {
            Discovery::Ready(ReadyDiscovery {
                options: options.clone(),
                pages_read: 1,
                quarantines: Vec::new(),
            })
        })
        .collect();
    let mut replay = decode(&mut encode(&plan)?)?;
    fixture
        .require(&mut replay, &fixture.fetcher)
        .map_err(crate::restate_services::tests::sdk_error)?;
    replay.reserve_execution()?;
    let charged = MAX_PLAN_ALLOCATIONS
        .checked_sub(replay.allocation.remaining()?)
        .ok_or("fixture charged allocation")?;
    check!(charged > 536_870_912);
    check!(charged < MAX_PLAN_ALLOCATIONS);
    Ok(())
}

#[test]
fn syntactically_valid_forged_work_id_refuses_actual_rebinding() -> TestResult {
    let fixture = Fixture::new()?;
    let mut plan = fixture.plan()?;
    plan.targets.first_mut().ok_or("fixture target")?.work_id =
        AcquisitionWorkId::parse(&"f".repeat(64))?;
    let mut replay = decode(&mut encode(&plan)?)?;
    let result = fixture.require(&mut replay, &fixture.fetcher);
    let error = result.err().ok_or("forged WorkId was accepted")?;
    check!(crate::restate_services::tests::sdk_error(error)
        .to_string()
        .contains("actual work binding"));
    Ok(())
}

#[test]
fn changed_actual_effective_user_agent_refuses_replayed_work() -> TestResult {
    let fixture = Fixture::new()?;
    let mut replay = decode(&mut encode(&fixture.plan()?)?)?;
    let changed = Arc::new(fetcher(&fixture.store, "different-fixture-agent")?);
    let error = fixture
        .require(&mut replay, &changed)
        .err()
        .ok_or("changed request was accepted")?;
    check!(crate::restate_services::tests::sdk_error(error)
        .to_string()
        .contains("actual work binding"));
    Ok(())
}
