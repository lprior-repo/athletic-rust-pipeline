use super::super::{build, JurisdictionFacts};
use crate::restate_services::plan::{self, BrowserLaneState};
use crate::restate_services::wire::{JurisdictionState, NationalRequest, SourcePlan};
use crate::restate_services::ExportGeneration;
use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_reconcile::identity::WorkflowIdentity;
use census_store::PublicResourceDeclarationBindingV1;

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) struct Fixture {
    pub(super) manifest: RunManifest,
    pub(super) capture: AcquisitionDigest,
    pub(super) bound: NationalRequest,
    pub(super) facts: Vec<JurisdictionFacts>,
}

impl Fixture {
    pub(super) fn new() -> TestResult<Self> {
        let mut bound = crate::restate_services::national::publication_tests::fixtures::request()?;
        bound.jurisdictions = sites();
        bound.history = HistoryWindow::new(2023, 2026, "2026-10-21")?;
        bound.observed_on = Some("2026-10-20".to_string());
        bound.limit_per_state = Some(19);
        bound.concurrency = 4;
        bound.source_parallelism = 3;
        bound.authorized_hosts = vec!["mi.milesplit.com".to_string()];
        bound.pass_budget = 96;
        bound.pass_delay_seconds = 900;
        bound.generation = Some(ExportGeneration::parse("five-state-builder-test")?);
        let store = bound
            .export_job
            .require_selected_publication()
            .store_identity()
            .clone();
        let manifest = RunManifest {
            store_identity: store.as_str().to_string(),
            run: CensusRun::new(bound.season, bound.revision.get()).ok_or("run")?,
            cohort: GradYear::CO2027,
            jurisdictions: sites(),
        };
        let capture = digest(&CaptureQualification::V2(CaptureQualificationV2::new(
            store,
        )))?;
        let facts = sites()
            .into_iter()
            .map(|site| owner(&bound, site))
            .collect::<TestResult<_>>()?;
        Ok(Self {
            manifest,
            capture,
            bound,
            facts,
        })
    }

    pub(super) fn binding(&self) -> TestResult<PublicResourceDeclarationBindingV1> {
        Ok(build(
            &self.manifest,
            &self.capture,
            &self.bound,
            &self.facts,
        )?)
    }

    pub(super) fn first_mut(&mut self) -> TestResult<&mut JurisdictionFacts> {
        self.facts.first_mut().ok_or_else(|| "first owner".into())
    }

    pub(super) fn plan_mut(&mut self) -> TestResult<&mut SourcePlan> {
        self.first_mut()?
            .state
            .plan
            .as_mut()
            .ok_or_else(|| "source plan".into())
    }

    pub(super) fn witness(&self) -> TestResult<String> {
        let facts = self
            .facts
            .iter()
            .map(|fact| {
                (
                    fact.site,
                    &fact.state,
                    &fact.directory_request,
                    &fact.robots_request,
                )
            })
            .collect::<Vec<_>>();
        Ok(serialized_digest(&(
            &self.manifest,
            &self.capture,
            &self.bound,
            facts,
        ))?)
    }

    pub(super) fn refused_without_input_changes(&self) -> TestResult {
        let before = self.witness()?;
        check!(build(&self.manifest, &self.capture, &self.bound, &self.facts).is_err());
        check!(eq; self.witness()?, before);
        Ok(())
    }
}

pub(super) fn sites() -> Vec<UsJurisdiction> {
    vec![
        UsJurisdiction::Michigan,
        UsJurisdiction::Minnesota,
        UsJurisdiction::Iowa,
        UsJurisdiction::Illinois,
        UsJurisdiction::Colorado,
    ]
}

pub(super) fn digest(value: &(impl serde::Serialize + ?Sized)) -> TestResult<AcquisitionDigest> {
    Ok(AcquisitionDigest::parse(&serialized_digest(value)?)?)
}

pub(super) fn owner(
    bound: &NationalRequest,
    site: UsJurisdiction,
) -> TestResult<JurisdictionFacts> {
    let source = census_crawl::milesplit::Site::for_jurisdiction(site);
    let lane = BrowserLaneState::Absent;
    Ok(JurisdictionFacts {
        site,
        state: JurisdictionState {
            identity: WorkflowIdentity::jurisdiction(site, bound.season, bound.revision)
                .to_string(),
            plan: Some(SourcePlan::of(
                &plan::plan(site, lane),
                plan::compute_plan_fingerprint(site, bound.season, bound.revision, lane),
            )),
            history_window: Some(bound.history),
            ..JurisdictionState::default()
        },
        directory_request: request(&source.teams_url())?,
        robots_request: request(&format!("https://{}/robots.txt", source.host()))?,
    })
}

pub(super) fn request(url: &str) -> TestResult<RequestRepresentationV1> {
    let user_agent = "Census/service-declaration-test";
    Ok(RequestRepresentationV1::normalized_get(
        url,
        vec![
            ("accept".to_string(), "*/*".to_string()),
            ("accept-encoding".to_string(), "identity".to_string()),
            ("user-agent".to_string(), user_agent.to_string()),
        ],
        user_agent,
    )?)
}
