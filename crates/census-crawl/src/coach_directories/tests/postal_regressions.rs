#[path = "postal_regressions/cases.rs"]
mod cases;
#[path = "postal_regressions/ownership.rs"]
mod ownership;
#[path = "postal_regressions/pagination.rs"]
mod pagination;

use super::super::{collect, directory_page_url, summary_url, Options};
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use crate::net::{FetchOptions, Fetcher};
use crate::AdapterContext;
use census_domain::model::{CanonicalCoach, CanonicalSchool, SchoolYear, SourceNamespace};
use census_domain::school_directory::SourceLabel;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

const DIRECTORY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/coach_directories/nchsaa_directory_p1.json"
));
const SUMMARY: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/coach_directories/nc_staff_summary_zcum49.json"
));
const SCHOOL_NAME: &str = "A.C. Reynolds High School";
const OBSERVED: &str = "2026-10-01T12:34:56Z";
const CAPTURED: &str = "2026-09-27T00:00:00Z";

struct FixtureRun {
    _directory: tempfile::TempDir,
    store: Store,
    fetcher: Fetcher,
}

impl FixtureRun {
    fn new(directory: &[u8], summary: &[u8]) -> Result<Self, Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path().join("store"))?;
        let fetcher = Fetcher::new(
            root.path().join("http"),
            None,
            Duration::from_millis(1),
            std::collections::HashMap::new(),
            vec![],
        )?
        .with_offline(true);
        seed(&fetcher, &directory_page_url("NCHSAA", 1), directory)?;
        seed(&fetcher, &summary_url("ZCUM49"), summary)?;
        Ok(Self {
            _directory: root,
            store,
            fetcher,
        })
    }

    async fn collect(&self) -> Result<crate::AdapterReport, Box<dyn std::error::Error>> {
        let context = AdapterContext {
            fetcher: &self.fetcher,
            store: &self.store,
            refresh: false,
            school_year: SchoolYear::new(2026).ok_or("valid fixture school year")?,
            observed_on: OBSERVED.to_string(),
            performance_as_of: chrono::NaiveDate::parse_from_str("2026-10-01", "%Y-%m-%d")?,
            recording: None,
        };
        Ok(collect(
            &context,
            &Options {
                states: vec![UsJurisdiction::NorthCarolina],
                school_names: vec![SCHOOL_NAME.to_string()],
                observed_on: "2025-01-01".to_string(),
                ..Options::default()
            },
        )
        .await?)
    }

    fn school(&self) -> Result<CanonicalSchool, Box<dyn std::error::Error>> {
        let schools = self.store.scan::<CanonicalSchool>(Table::Schools)?;
        check!(eq; schools.len(), 1);
        Ok(schools
            .into_iter()
            .next()
            .ok_or("retained directory school")?)
    }

    fn coaches(&self) -> Result<Vec<CanonicalCoach>, Box<dyn std::error::Error>> {
        Ok(self.store.scan(Table::Coaches)?)
    }
}

fn seed(fetcher: &Fetcher, url: &str, body: &[u8]) -> TestResult {
    let key = Fetcher::key_for("GET", url, "");
    let (body_path, meta_path) = fetcher.cache_paths(&key);
    write_cache(
        &body_path,
        &meta_path,
        body,
        &CacheMeta {
        redirects: Vec::new(),
            representation: crate::net::RepresentationHeaders::default(),
            url: url.to_string(),
            response_url: None,
            method: "GET".to_string(),
            status: 200,
            content_digest: content_digest(body),
            bytes: body.len(),
            fetched_at: CAPTURED.to_string(),
            etag: None,
            last_modified: None,
            content_type: Some("application/json".to_string()),
        },
    )?;
    Ok(())
}

fn changed_summary(
    change: impl FnOnce(&mut serde_json::Value),
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut value: serde_json::Value = serde_json::from_slice(SUMMARY)?;
    change(&mut value);
    Ok(serde_json::to_vec(&value)?)
}

fn changed_directory(
    change: impl FnOnce(&mut serde_json::Value),
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut value: serde_json::Value = serde_json::from_slice(DIRECTORY)?;
    let row = value
        .get_mut("results")
        .and_then(serde_json::Value::as_array_mut)
        .and_then(|rows| rows.first_mut())
        .ok_or("directory capture first school")?;
    change(row);
    Ok(serde_json::to_vec(&value)?)
}

#[test]
fn captured_zcum49_keeps_both_source_owned_addresses_and_raw_capture_provenance() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new(DIRECTORY, SUMMARY)?;
            let report = run.collect().await?;
            check!(eq; (report.rows, report.errors), (1, 0));
            let school = run.school()?;
            check!(eq; school.name, SCHOOL_NAME);
            check!(eq; school.postal_addresses.len(), 2);
            let claim = school
                .postal_addresses
                .iter()
                .find(|claim| {
                    claim.evidence().source.url.as_deref() == Some(summary_url("ZCUM49").as_str())
                })
                .ok_or("summary-owned postal claim")?;
            check!(eq;
                claim.address().line1().map(|line| line.as_str()),
                Some("1 Rocket Drive")
            );
            check!(eq; claim.address().line2(), None);
            check!(eq;
                claim.address().city().map(|city| city.as_str()),
                Some("Asheville")
            );
            check!(eq; claim.address().state(), Some(UsJurisdiction::NorthCarolina));
            check!(eq; claim.address().zip().map(|zip| zip.code()), Some("28803"));
            check!(eq;
                claim.owner().namespace,
                SourceNamespace::association_school("coach_directories")
            );
            check!(eq; claim.owner().id, "ZCUM49");
            check!(eq;
                claim.source_label(),
                &SourceLabel::AthleticAssociation {
                    state: UsJurisdiction::NorthCarolina
                }
            );
            check!(eq; claim.evidence().observed_on, CAPTURED);
            check!(eq; claim.capture_sha256(), content_digest(SUMMARY));
            let directory_claim = school
                .postal_addresses
                .iter()
                .find(|claim| {
                    claim.evidence().source.url.as_deref()
                        == Some(directory_page_url("NCHSAA", 1).as_str())
                })
                .ok_or("independent directory capture claim")?;
            check!(eq; directory_claim.address().zip(), None);
            check!(eq; directory_claim.capture_sha256(), content_digest(DIRECTORY));
            check!(eq; directory_claim.evidence().observed_on, CAPTURED);
            check!(eq; run.coaches()?.len(), 16);
            check!(run.coaches()?.iter().all(|coach| coach.school == school.id));
            let cached = run
                .fetcher
                .get(&summary_url("ZCUM49"), &FetchOptions::default())
                .await?;
            check!(eq; cached.body, SUMMARY);
            check!(eq; cached.fetched_at, CAPTURED);
            Ok(())
        })
}

#[test]
fn foreign_summary_never_attaches_coaches_aliases_or_address_and_remains_unfinished() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let body = changed_summary(|summary| {
                summary["id"] = "another-organization".into();
                summary["name"] = "Foreign School".into();
                summary["address"]["address1"] = "999 Foreign Road".into();
            })?;
            let run = FixtureRun::new(DIRECTORY, &body)?;
            let report = run.collect().await?;
            check!(eq; (report.rows, report.errors), (0, 1));
            let school = run.school()?;
            check!(eq; school.name, SCHOOL_NAME);
            check!(eq; school.city.as_deref(), Some("Asheville"));
            check!(eq; school.classification.as_deref(), Some("6A"));
            check!(eq; school.aliases, Vec::<String>::new());
            check!(eq; school.postal_addresses.len(), 1);
            check!(eq;
                school.postal_addresses.first().ok_or("retained directory address")?
                    .address()
                    .line1()
                    .map(|line| line.as_str()),
                Some("1 Rocket Drive")
            );
            check!(eq; run.coaches()?, Vec::<CanonicalCoach>::new());
            seed(&run.fetcher, &summary_url("ZCUM49"), SUMMARY)?;
            let recovered = run.collect().await?;
            check!(eq; (recovered.rows, recovered.errors), (1, 0));
            check!(eq; run.school()?.postal_addresses.len(), 2);
            check!(eq; run.coaches()?.len(), 16);
            Ok(())
        })
}

#[test]
fn alternate_published_streets_are_not_ranked_or_discarded() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let body = changed_summary(|summary| {
                summary["address"]["address1"] = "2 Published Alternate Drive".into();
                summary["address"]["address2"] = "Suite 7".into();
                summary["address"]["zip"] = "02803-0042".into();
            })?;
            let run = FixtureRun::new(DIRECTORY, &body)?;
            check!(eq; run.collect().await?.errors, 0);
            let school = run.school()?;
            let addresses: Vec<_> = school
                .postal_addresses
                .iter()
                .map(|claim| claim.address().line1().map(|line| line.as_str()))
                .collect();
            check!(eq;
                addresses,
                vec![Some("1 Rocket Drive"), Some("2 Published Alternate Drive")]
            );
            let claim = school
                .postal_addresses
                .get(1)
                .ok_or("alternate published claim")?;
            check!(eq;
                claim.address().line2().map(|line| line.as_str()),
                Some("Suite 7")
            );
            check!(eq; claim.address().zip().map(|zip| zip.code()), Some("02803"));
            check!(eq;
                claim.address().zip().and_then(|zip| zip.plus4()),
                Some("0042")
            );
            check!(eq; claim.capture_sha256(), content_digest(&body));
            Ok(())
        })
}
