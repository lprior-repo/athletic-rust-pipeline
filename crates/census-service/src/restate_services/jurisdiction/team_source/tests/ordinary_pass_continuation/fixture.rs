use super::*;
use crate::restate_services::wire::{
    NationalRequest, NationalRequestBinding, RefusedSource, SatisfiedSource,
    TeamsContinuationOutcome, TeamsSourceRequest,
};
use crate::restate_services::{CaptureExecution, Jobs};
use census_crawl::net::Fetcher;
use census_domain::model::serialized_digest;
use census_store::{clock::SystemClock, Store};
use std::{sync::Arc, time::Duration};

mod source;

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    pub(super) store: Arc<Store>,
    pub(super) region: Arc<crate::spawn::Spawner>,
    pub(super) owner: JurisdictionCensus,
    pub(super) source: TeamsSourceRequest,
    pub(super) state: JurisdictionState,
    pub(super) national: NationalRequest,
    pub(super) current: Option<TeamsContinuationOutcome>,
}

impl Fixture {
    pub(super) fn new(mode: &str) -> TestResult<Self> {
        let root = tempfile::tempdir()?;
        Store::create_current_qualification_v3(root.path().join("store"))?;
        let (store, writer) =
            Store::open_current_qualification_v3(root.path().join("store"))?.into_parts();
        let mut source = request()?;
        source.source = "milesplit".into();
        public_resources_fixture::bind(&store, &source)?;
        let fetcher = Fetcher::new(
            store.http_cache_dir(),
            None,
            Duration::ZERO,
            Default::default(),
            Vec::new(),
        )?
        .with_origin_locks(root.path().join("isolated-locks"))
        .with_offline(true);
        public_resources_fixture::declare(&store, &writer, &source, &fetcher)?;
        let national = public_resources_fixture::national(&store, &source)?;
        let (state, current) = source::seed(&store, &source, mode)?;
        let store = Arc::new(store);
        let region = Arc::new(crate::spawn::Spawner::new());
        let owner = JurisdictionCensus::new(
            Arc::clone(&store),
            Arc::new(SystemClock),
            None,
            CaptureExecution::FreshQualification(Arc::new(writer)),
            Jobs::new(Arc::clone(&store), Arc::clone(&region)),
        );
        let mut result = Self {
            _root: root,
            store,
            region,
            owner,
            source,
            state,
            national,
            current,
        };
        result.configure(mode)?;
        Ok(result)
    }

    fn configure(&mut self, mode: &str) -> TestResult {
        match mode {
            "foreign_bound" => self.national.refresh = !self.national.refresh,
            "foreign_plan" => {
                self.state.plan.as_mut().ok_or("plan absent")?.fingerprint = "a".repeat(64)
            }
            "wrong_digest" => {
                self.current
                    .as_mut()
                    .ok_or("current absent")?
                    .original_outcome_digest = "b".repeat(64)
            }
            "unrelated_parent" => source::alter_parent(&mut self.state)?,
            value if value.starts_with("inventory_") => self.ambiguate_inventory(value)?,
            _ => {}
        }
        Ok(())
    }

    fn ambiguate_inventory(&mut self, mode: &str) -> TestResult {
        let plan = self.state.plan.as_mut().ok_or("plan absent")?;
        let refused = |slug: &str| RefusedSource {
            slug: slug.into(),
            reason: "controlled contradictory retained disposition".into(),
            kind: crate::restate_services::plan::RefusalKind::EngineeringGap,
        };
        let satisfied = |slug: &str| SatisfiedSource {
            slug: slug.into(),
            evidence_digest: "c".repeat(64),
        };
        match mode {
            "inventory_sweep_refused" => plan.refused.push(refused("ihsa")),
            "inventory_sweep_satisfied" => plan.satisfied.push(satisfied("ihsa")),
            "inventory_sweep_duplicate" => plan.sweepable.push("ihsa".into()),
            "inventory_unselected_duplicate" => {
                plan.sweepable
                    .extend(["sidearm_staff".into(), "sidearm_staff".into()]);
            }
            "inventory_refused_duplicate" => {
                plan.refused.extend([refused("tfrrs"), refused("tfrrs")]);
            }
            "inventory_satisfied_duplicate" => {
                plan.satisfied
                    .extend([satisfied("nces"), satisfied("nces")]);
            }
            "inventory_refused_satisfied" => {
                plan.refused.push(refused("tfrrs"));
                plan.satisfied.push(satisfied("tfrrs"));
            }
            _ => return Err("unknown inventory mutation".into()),
        }
        check!(eq; plan.fingerprint, public_resources_fixture::fingerprint(&self.source));
        Ok(())
    }

    pub(super) fn response(&self, call: &Frame, mode: &str) -> TestResult<Vec<u8>> {
        if call.text(1)? == "NationalCensus" {
            check!(eq; call.text(2)?, "request");
            check!(eq; call.text(5)?, census_reconcile::identity::WorkflowIdentity::national(
                self.national.season, self.national.revision, &self.national.jurisdictions,
            ).to_string());
            return Ok(serde_json::to_vec(&NationalRequestBinding::Bound(
                Box::new(self.national.clone()),
            ))?);
        }
        check!(eq; call.text(1)?, "TeamsSource");
        check!(codec::field(&call.payload, 3)?.is_none_or(<[u8]>::is_empty));
        let mut source = self.source.clone();
        if call.text(5)? != key(&source) {
            source.source = "ihsa".into();
            check!(eq; call.text(5)?, key(&source));
        }
        if call.text(2)? == "current" {
            check!(eq; source.source, "milesplit");
            return Ok(serde_json::to_vec(&self.current)?);
        }
        check!(eq; call.text(2)?, "state");
        let value = if source.source == "ihsa" && mode == "unsettled_other" {
            None
        } else {
            ledger::status(&self.store, &key(&source), Some(&source))?
        };
        Ok(serde_json::to_vec(&value)?)
    }

    pub(super) fn snapshot(&self) -> TestResult<serde_json::Value> {
        let mut rows = Vec::new();
        for phase in [ledger::PHASE, ledger::origin_continuation::PHASE] {
            let mut keys: Vec<_> = self.store.journal_keys(phase)?.into_iter().collect();
            keys.sort();
            for key in keys {
                rows.push((phase, key.clone(), self.store.journal_payload(phase, &key)?));
            }
        }
        let snapshot = self.store.snapshot();
        Ok(serde_json::json!({
            "rows": rows,
            "sequence": snapshot.sequence(),
            "evidence": snapshot.evidence_generation(),
            "derived": snapshot.derived_generation(),
            "tables": snapshot.tables_digest(&census_store::Table::ALL)?,
        }))
    }
}
