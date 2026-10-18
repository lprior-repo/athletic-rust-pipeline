use super::*;
use census_crawl::milesplit::{
    collect_result_sets, ResultSetOptions, ResultSetRef, ResultSetRequest,
};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::{AdapterContext, AdapterReport, CaptureContext, CrawlResult};
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, QueryProfileV1, SchoolYear, SourceIdentity, SourceNamespace,
};
use census_domain::participation_candidates::participation_policy_v4;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

mod cache;

const TROY: &[u8] =
    include_bytes!("../../../../../../census-crawl/src/milesplit/owned/fixtures/troy_725218.json");
pub(super) const RAW: &[u8] = include_bytes!("../../../../../../census-crawl/tests/fixtures/milesplit/troy_725218_rs1266814_raw_projection.html");

pub(super) struct Fixture {
    pub(super) _directory: tempfile::TempDir,
    pub(super) store: Store,
    pub(super) performance: CanonicalPerformance,
    pub(super) profile: QueryProfileV1,
}

impl Fixture {
    pub(super) async fn prepare(synthetic_division: Option<&str>) -> TestResult<Self> {
        Self::prepare_raw(synthetic_division, RAW).await
    }

    pub(super) async fn prepare_raw(
        synthetic_division: Option<&str>,
        raw: &[u8],
    ) -> TestResult<Self> {
        let dir = tempfile::tempdir()?;
        let root = dir.path().join("store");
        Store::create_public_identity_qualification_v3(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
        let fetcher = Fetcher::new(
            dir.path().join("http"),
            None,
            std::time::Duration::ZERO,
            HashMap::new(),
            vec!["milesplit.com".into()],
        )?
        .with_offline(true);
        let reference =
            ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266814/raw")
                .ok_or("result set")?;
        for team in ["38332", "4912"] {
            let mut school = CanonicalSchool::new(UsJurisdiction::Alabama, team, team, None).0;
            school
                .source_identities
                .push(SourceIdentity::new(SourceNamespace::MilesplitSchool, team));
            store.append(Table::Schools, &school)?;
        }
        let body = synthetic_body(synthetic_division)?;
        let request = census_crawl::milesplit::owned_meet_request(
            &fetcher,
            &reference,
            &FetchOptions::default(),
        )?;
        cache::seed(&fetcher, request.original_url(), &body)?;
        cache::seed(&fetcher, &reference.url, raw)?;
        let ctx = AdapterContext {
            capture: CaptureContext::LegacyMetadata,
            fetcher: &fetcher,
            store: &store,
            refresh: false,
            school_year: SchoolYear::new(2026).ok_or("school year")?,
            observed_on: "2099-01-01".into(),
            performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 10, 7).ok_or("date")?,
            recording: None,
        };
        let options = ResultSetOptions {
            urls: vec![ResultSetRequest {
                url: reference.url.clone(),
                jurisdiction: UsJurisdiction::Alabama,
            }],
        };
        let acquisition: Pin<Box<dyn Future<Output = CrawlResult<AdapterReport>> + '_>> =
            Box::pin(collect_result_sets(&ctx, &options));
        let report = acquisition.await?;
        assert_eq!(report.errors, 0, "{:?}", report.notes);
        let performance = store
            .scan::<CanonicalPerformance>(Table::Performances)?
            .into_iter()
            .find(|row| row.source_key == "milesplit_result:201782263")
            .ok_or("owned performance")?;
        let profile = super::profile::profile(participation_policy_v4()?)?;
        store.store_identity()?;
        store.register_profile(&profile)?;
        for athlete in store.snapshot().athletes()? {
            writer.establish_source_bound(
                &store,
                &format!("source-bound-participation-{}", athlete.id),
                &athlete.id.cast(),
                store.snapshot().revision(),
                &writer.head(&store)?,
            )?;
        }
        Ok(Self {
            _directory: dir,
            store,
            performance,
            profile,
        })
    }

    pub(super) fn dataset(
        &self,
        snapshot: &census_store::StoreSnapshot<'_>,
    ) -> TestResult<ExportDataset> {
        let mut dataset =
            ExportDataset::load_public_identity_snapshot_fixture(&self.store, snapshot)?;
        dataset.bind_selection_for_test(self.profile.id()?, self.profile.input().as_of.clone())?;
        dataset.bind_participation_rule_for_test(
            self.profile.input().policies.participation.clone(),
        )?;
        dataset.bind_public_identity_fixture_from_snapshot(snapshot)?;
        Ok(dataset)
    }

    pub(super) fn assess(&self) -> TestResult<PinnedParticipation> {
        let snapshot = self.store.snapshot();
        let dataset = self.dataset(&snapshot)?;
        self.read(&dataset, &snapshot, &self.profile)
    }
    pub(super) fn objects(&self) -> std::path::PathBuf {
        self._directory.path().join("proofs")
    }

    pub(super) fn read(
        &self,
        dataset: &ExportDataset,
        snapshot: &census_store::StoreSnapshot<'_>,
        profile: &QueryProfileV1,
    ) -> TestResult<PinnedParticipation> {
        let directory = self.objects();
        std::fs::create_dir_all(&directory)?;
        let athletes =
            crate::export::qualification::physical_observations::capture::<CanonicalAthlete>(
                dataset,
                snapshot,
                Table::Athletes,
                &directory,
                observations::BUDGET,
            )?;
        let affiliations =
            crate::export::qualification::current_affiliation::PinnedCurrentAffiliations::read(
                dataset, snapshot,
            )?;
        Ok(PinnedParticipation::read(
            dataset,
            snapshot,
            profile,
            &directory,
            &athletes,
            athletes.fingerprint(),
            affiliations.roster_context(),
            None,
        )?)
    }
}

fn synthetic_body(division: Option<&str>) -> TestResult<Vec<u8>> {
    let Some(division) = division else {
        return Ok(TROY.to_vec());
    };
    let mut body: Value = serde_json::from_slice(TROY)?;
    let rows = body
        .get_mut("data")
        .and_then(Value::as_array_mut)
        .ok_or("synthetic data")?;
    rows.truncate(1);
    rows.first_mut()
        .and_then(Value::as_object_mut)
        .ok_or("synthetic first row")?
        .insert("divisionName".into(), json!(division));
    Ok(serde_json::to_vec(&body)?)
}
