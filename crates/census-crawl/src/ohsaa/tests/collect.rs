use super::{capture, fixture_search_dublin, fixture_sports_dublin, AD_FETCHED, SPORTS_FETCHED};
use crate::net::cache::{write_cache, CacheMeta};
use crate::net::Fetcher;
use crate::ohsaa::{collect, Options, SearchResult, HOST};
use crate::{AdapterContext, AdapterReport, Recording};
use census_domain::model::{CanonicalCoach, CanonicalSchool, SchoolYear, SourceObservation};
use census_store::{Store, Table};
use std::collections::HashMap;
use std::time::Duration;

mod changed;
mod failures;
mod owner;
mod refused;
mod replay;

const OUTCOMES: &str = "ohsaa_school_outcomes_v1";
const CAPTURES: &str = "ohsaa_captures_v1";

struct FixtureRun {
    _root: tempfile::TempDir,
    store: Store,
    fetcher: Fetcher,
}

impl FixtureRun {
    fn new(ad: Option<&str>) -> anyhow::Result<Self> {
        Self::new_pages(Some(fixture_sports_dublin()), ad)
    }

    fn new_pages(sports: Option<&str>, ad: Option<&str>) -> anyhow::Result<Self> {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path().join("store"))?;
        let fetcher = Fetcher::new(
            root.path().join("http"),
            None,
            Duration::from_millis(1),
            HashMap::new(),
            vec![],
        )?
        .with_offline(true);
        let run = Self {
            _root: root,
            store,
            fetcher,
        };
        run.seed(&search_url(), fixture_search_dublin(), SPORTS_FETCHED)?;
        if let Some(body) = sports {
            run.seed(&dublin().sports_url(), body, SPORTS_FETCHED)?;
        }
        if let Some(body) = ad {
            run.seed(&dublin().ad_url(), body, AD_FETCHED)?;
        }
        Ok(run)
    }

    fn seed(&self, url: &str, body: &str, fetched_at: &str) -> anyhow::Result<()> {
        let captured = capture(url.to_string(), body, fetched_at);
        let key = Fetcher::key_for("GET", url, "");
        let (body_path, meta_path) = self.fetcher.cache_paths(&key);
        write_cache(
            &body_path,
            &meta_path,
            body.as_bytes(),
            &CacheMeta {
        redirects: Vec::new(),
                representation: crate::net::RepresentationHeaders::default(),
                url: captured.url,
                response_url: captured.response_url,
                method: captured.method,
                status: captured.status,
                content_digest: captured.content_digest,
                bytes: captured.bytes,
                fetched_at: captured.fetched_at,
                etag: None,
                last_modified: None,
                content_type: captured.content_type,
            },
        )?;
        Ok(())
    }

    fn context<'a>(
        &'a self,
        evaluated_on: &str,
        recording: Option<&'a Recording>,
    ) -> anyhow::Result<AdapterContext<'a>> {
        Ok(AdapterContext {
            fetcher: &self.fetcher,
            store: &self.store,
            refresh: false,
            school_year: SchoolYear::new(2026).ok_or_else(|| anyhow::anyhow!("school year"))?,
            observed_on: evaluated_on.to_string(),
            performance_as_of: chrono::NaiveDate::parse_from_str(
                evaluated_on
                    .get(..10)
                    .ok_or_else(|| anyhow::anyhow!("evaluation date"))?,
                "%Y-%m-%d",
            )?,
            recording,
        })
    }

    async fn run(
        &self,
        evaluated_on: &str,
        recording: Option<&Recording>,
    ) -> anyhow::Result<AdapterReport> {
        let context = self.context(evaluated_on, recording)?;
        Ok(collect(
            &context,
            &Options {
                school_names: vec!["Dublin Coffman".to_string()],
                observed_on: evaluated_on.to_string(),
                ..Options::default()
            },
        )
        .await?)
    }

    fn counts(&self) -> anyhow::Result<(u64, u64, u64)> {
        Ok((
            self.store.walk_table(Table::Schools)?.rows,
            self.store.walk_table(Table::Coaches)?.rows,
            self.store.walk_table(Table::SourceObservations)?.rows,
        ))
    }

    fn outcome(&self) -> anyhow::Result<serde_json::Value> {
        self.store
            .journal_payloads(OUTCOMES)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow::anyhow!("school outcome absent"))
    }
}

fn dublin() -> SearchResult {
    SearchResult {
        name: "DUBLIN COFFMAN".to_string(),
        city: "Dublin".to_string(),
        ohsaa_id: "474".to_string(),
    }
}

fn search_url() -> String {
    format!("{HOST}/Outside/SearchSchool?Name=Dublin%20Coffman")
}

#[test]
fn cached_capture_dates_are_not_replaced_by_newer_evaluation_dates(
) -> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    let run = FixtureRun::new(Some(super::fixture_ad_dublin()))?;
    let report = run.run("2026-10-02", None).await?;
    check!(eq; (report.rows, report.errors, report.with_email), (1, 0, 5));
    check!(eq; report.requests, 0);
    check!(eq; run.counts()?, (1, 5, 1));
    let schools: Vec<CanonicalSchool> = run.store.scan(Table::Schools)?;
    let school = schools.first().ok_or("captured school")?;
    check!(eq; school.evidence.first().ok_or("school evidence")?.observed_on, SPORTS_FETCHED);
    let observations: Vec<SourceObservation> = run.store.scan(Table::SourceObservations)?;
    check!(eq; observations.first().ok_or("source observation")?.observed_on(), SPORTS_FETCHED);
    let coaches: Vec<CanonicalCoach> = run.store.scan(Table::Coaches)?;
    assert_coach_captures(&coaches, school)?;
    let outcome = run.outcome()?;
    check!(eq; outcome["state"], "complete");
    check!(eq; outcome["evaluated_on"], "2026-10-02");
    check!(eq; outcome["sports_capture"]["acquired_at"], SPORTS_FETCHED);
    check!(eq; outcome["ad_capture"]["acquired_at"], AD_FETCHED);
    let receipts = run.store.journal_payloads("ohsaa_schools")?;
    check!(eq; receipts.len(), 1);
    let receipt = receipts.first().ok_or("completed school receipt")?;
    check!(eq;
        receipt["sports_capture"]["capture_url"],
        dublin().sports_url()
    );
    check!(eq; receipt["ad_capture"]["capture_url"], dublin().ad_url());
    Ok(())
    })
}

fn assert_coach_captures(
    coaches: &[CanonicalCoach],
    school: &CanonicalSchool,
) -> Result<(), Box<dyn std::error::Error>> {
    coaches.iter().try_for_each(|coach| {
        let director = coach.role == census_domain::model::CoachRole::AthleticDirector;
        let expected = if director {
            capture(dublin().ad_url(), super::fixture_ad_dublin(), AD_FETCHED)
        } else {
            capture(
                dublin().sports_url(),
                fixture_sports_dublin(),
                SPORTS_FETCHED,
            )
        };
        check!(eq; coach.school, school.id);
        let current = coach.tenure_evidence.first().ok_or("capture-bound current appointment")?;
        check!(eq; current.tenure, census_domain::model::CoachTenure::Current { school_year: SchoolYear::new(2026).ok_or("school year")? });
        check!(eq; current.source.url.as_deref(), Some(expected.url.as_str()));
        check!(eq; current.retrieved_at, expected.fetched_at);
        check!(eq; current.source_sha256, expected.content_digest);
        let claim = current.claim.as_ref().ok_or("staff contact claim")?;
        check!(eq; claim.coach, coach.id);
        check!(eq; claim.school, coach.school);
        check!(eq; claim.role, coach.role);
        check!(eq; claim.mailbox.as_deref(), coach.professional_email.as_deref().or(coach.personal_email.as_deref()));
        let evidence = coach.evidence.first().ok_or("coach capture evidence")?;
        check!(eq; evidence.observed_on, expected.fetched_at);
        check!(eq; evidence.source.url.as_deref(), Some(expected.url.as_str()));
        let note: serde_json::Value = serde_json::from_str(
            evidence
                .note
                .as_deref()
                .ok_or_else(|| anyhow::anyhow!("capture note"))?,
        )?;
        check!(eq; note["sha256"], expected.content_digest);
        check!(eq; note["acquired_at"], expected.fetched_at);
        Ok::<_, Box<dyn std::error::Error>>(())
    })?;
    Ok(())
}

fn structured_absent_ad() -> String {
    super::fixture_ad_dublin()
        .replace("Duane Sheldon", "N/A")
        .replace("sheldon_duane@dublinschools.net", "")
}
