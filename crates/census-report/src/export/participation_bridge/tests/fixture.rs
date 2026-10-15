use super::*;
use census_crawl::milesplit::{
    collect_result_sets, read_retained_owned_result, read_retained_result_metadata,
    CaptureReadBudget, OwnedAcquisitionRef, RawMetadataArchiveRef, ResultSetOptions, ResultSetRef,
    ResultSetRequest,
};
use census_crawl::net::{FetchOptions, Fetcher};
use census_crawl::{AdapterContext, AdapterReport, CaptureContext, CrawlResult};
use census_domain::affiliation_candidates::DeclaredFrozenView;
use census_domain::model::{
    CanonicalSchool, ProfileId, QueryProfileV1, SchoolYear, SourceIdentity, SourceNamespace,
};
use census_domain::participation_candidates::{participation_policy_v3, CandidateBinding};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::future::Future;
use std::num::NonZeroU64;
use std::pin::Pin;

mod cache;

const TROY: &[u8] =
    include_bytes!("../../../../../census-crawl/src/milesplit/owned/fixtures/troy_725218.json");
pub(super) const RAW: &[u8] = include_bytes!("../../../../../census-crawl/tests/fixtures/milesplit/troy_725218_rs1266814_raw_projection.html");

pub(super) struct Fixture {
    pub(super) source: VerifiedRetainedResultMetadata,
    pub(super) performance: CanonicalPerformance,
    pub(super) profile: QueryProfileV1,
    pub(super) profile_id: ProfileId,
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
        let store = Store::open(dir.path().join("store"))?;
        let _identity = store.store_identity()?;
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
        let source = read_source(&store, &reference)?;
        let performance = store
            .scan::<CanonicalPerformance>(Table::Performances)?
            .into_iter()
            .find(|row| row.source_key == "milesplit_result:201782263")
            .ok_or("owned performance")?;
        let profile = super::profile::profile(participation_policy_v3()?)?;
        Ok(Self {
            source,
            performance,
            profile_id: profile.id()?,
            profile,
        })
    }

    pub(super) fn query(&self) -> TestResult<CandidateQuery<'_>> {
        Ok(CandidateQuery {
            profile: &self.profile,
            binding: CandidateBinding {
                profile: &self.profile_id,
                participation_rule: &self.profile.input().policies.participation,
                frozen: DeclaredFrozenView {
                    lineage: ObjectRef::parse(self.source.store_identity().as_str())?,
                    revision: NonZeroU64::new(self.source.snapshot_sequence())
                        .ok_or("snapshot revision")?,
                    captured_at: chrono::DateTime::parse_from_rfc3339("2026-10-14T12:00:00Z")?
                        .with_timezone(&chrono::Utc),
                    closure_object: ObjectRef::parse("synthetic-test-closure")?,
                    family_inventory: ObjectRef::parse("synthetic-test-inventory")?,
                },
            },
        })
    }

    pub(super) fn assess(
        &self,
        cohort: SourceCohortAgreement,
    ) -> Result<SourceParticipationAssessment, SourceParticipationError> {
        let query = self
            .query()
            .map_err(|_| SourceParticipationError::ForeignSnapshot)?;
        assess_retained_source_participation(query, &self.performance, &self.source, cohort)
    }
}

fn synthetic_body(division: Option<&str>) -> TestResult<Vec<u8>> {
    let Some(division) = division else {
        return Ok(TROY.to_vec());
    };
    let mut body: Value = serde_json::from_slice(TROY)?;
    body.get_mut("data")
        .and_then(Value::as_array_mut)
        .and_then(|rows| rows.first_mut())
        .and_then(Value::as_object_mut)
        .ok_or("synthetic first row")?
        .insert("divisionName".into(), json!(division));
    Ok(serde_json::to_vec(&body)?)
}

fn read_source(
    store: &Store,
    reference: &ResultSetRef,
) -> TestResult<VerifiedRetainedResultMetadata> {
    let receipts = store.journal_payloads(census_crawl::milesplit::RESULT_SET_PHASE)?;
    let receipt = receipts
        .iter()
        .find(|value| {
            value.get("disposition").and_then(Value::as_str) == Some("projection_applied")
        })
        .ok_or("projection receipt")?;
    let owned = OwnedAcquisitionRef::parse(
        receipt
            .get("owned_capture_archive")
            .and_then(Value::as_str)
            .ok_or("owned archive")?,
    )?;
    let raw = RawMetadataArchiveRef::parse(
        receipt
            .get("raw_metadata_archive")
            .and_then(|value| value.get("archive_manifest"))
            .and_then(Value::as_str)
            .ok_or("raw archive")?,
    )?;
    let view = store.snapshot();
    let budget = CaptureReadBudget::new(1024 * 1024)?;
    let result = read_retained_owned_result(&view, &owned, "data[0]", budget)?;
    Ok(read_retained_result_metadata(
        &view, reference, &raw, &result, budget,
    )?)
}
