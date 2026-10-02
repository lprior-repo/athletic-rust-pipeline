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
    fn new(directory: &[u8], summary: &[u8]) -> Self {
        let root = tempfile::tempdir().expect("isolated fixture directory");
        let store = Store::open(root.path().join("store")).expect("isolated fixture store");
        let fetcher = Fetcher::new(
            root.path().join("http"),
            None,
            Duration::from_millis(1),
            std::collections::HashMap::new(),
            vec![],
        )
        .expect("fixture fetcher")
        .with_offline(true);
        seed(&fetcher, &directory_page_url("NCHSAA", 1), directory);
        seed(&fetcher, &summary_url("ZCUM49"), summary);
        Self {
            _directory: root,
            store,
            fetcher,
        }
    }

    async fn collect(&self) -> crate::AdapterReport {
        let context = AdapterContext {
            fetcher: &self.fetcher,
            store: &self.store,
            refresh: false,
            school_year: SchoolYear::new(2026).expect("school year"),
            observed_on: OBSERVED.to_string(),
            recording: None,
        };
        collect(
            &context,
            &Options {
                states: vec![UsJurisdiction::NorthCarolina],
                school_names: vec![SCHOOL_NAME.to_string()],
                observed_on: "2025-01-01".to_string(),
                ..Options::default()
            },
        )
        .await
        .expect("fixture acquisition report")
    }

    fn school(&self) -> CanonicalSchool {
        let schools = self
            .store
            .scan::<CanonicalSchool>(Table::Schools)
            .expect("school facts");
        assert_eq!(schools.len(), 1);
        schools
            .into_iter()
            .next()
            .expect("retained directory school")
    }

    fn coaches(&self) -> Vec<CanonicalCoach> {
        self.store.scan(Table::Coaches).expect("coach facts")
    }
}

fn seed(fetcher: &Fetcher, url: &str, body: &[u8]) {
    let key = Fetcher::key_for("GET", url, "");
    let (body_path, meta_path) = fetcher.cache_paths(&key);
    write_cache(
        &body_path,
        &meta_path,
        body,
        &CacheMeta {
            url: url.to_string(),
            method: "GET".to_string(),
            status: 200,
            content_digest: content_digest(body),
            bytes: body.len(),
            fetched_at: CAPTURED.to_string(),
            etag: None,
            last_modified: None,
            content_type: Some("application/json".to_string()),
        },
    )
    .expect("retained raw fixture capture");
}

fn changed_summary(change: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let mut value: serde_json::Value =
        serde_json::from_slice(SUMMARY).expect("summary capture JSON");
    change(&mut value);
    serde_json::to_vec(&value).expect("adversarial summary JSON")
}

fn changed_directory(change: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let mut value: serde_json::Value =
        serde_json::from_slice(DIRECTORY).expect("directory capture JSON");
    change(&mut value["results"][0]);
    serde_json::to_vec(&value).expect("adversarial directory JSON")
}

#[tokio::test]
async fn captured_zcum49_keeps_both_source_owned_addresses_and_raw_capture_provenance() {
    let run = FixtureRun::new(DIRECTORY, SUMMARY);
    let report = run.collect().await;
    assert_eq!((report.rows, report.errors), (1, 0));
    let school = run.school();
    assert_eq!(school.name, SCHOOL_NAME);
    assert_eq!(school.postal_addresses.len(), 2);
    let claim = school
        .postal_addresses
        .iter()
        .find(|claim| {
            claim.evidence().source.url.as_deref() == Some(summary_url("ZCUM49").as_str())
        })
        .expect("summary-owned postal claim");
    assert_eq!(
        claim.address().line1().map(|line| line.as_str()),
        Some("1 Rocket Drive")
    );
    assert_eq!(claim.address().line2(), None);
    assert_eq!(
        claim.address().city().map(|city| city.as_str()),
        Some("Asheville")
    );
    assert_eq!(claim.address().state(), Some(UsJurisdiction::NorthCarolina));
    assert_eq!(claim.address().zip().map(|zip| zip.code()), Some("28803"));
    assert_eq!(
        claim.owner().namespace,
        SourceNamespace::association_school("coach_directories")
    );
    assert_eq!(claim.owner().id, "ZCUM49");
    assert_eq!(
        claim.source_label(),
        &SourceLabel::AthleticAssociation {
            state: UsJurisdiction::NorthCarolina
        }
    );
    assert_eq!(claim.evidence().observed_on, OBSERVED);
    assert_eq!(claim.capture_sha256(), content_digest(SUMMARY));
    let directory_claim = school
        .postal_addresses
        .iter()
        .find(|claim| {
            claim.evidence().source.url.as_deref() == Some(directory_page_url("NCHSAA", 1).as_str())
        })
        .expect("independent directory capture claim");
    assert_eq!(directory_claim.address().zip(), None);
    assert_eq!(directory_claim.capture_sha256(), content_digest(DIRECTORY));
    assert_eq!(directory_claim.evidence().observed_on, OBSERVED);
    assert_eq!(run.coaches().len(), 16);
    assert!(run.coaches().iter().all(|coach| coach.school == school.id));
    let cached = run
        .fetcher
        .get(&summary_url("ZCUM49"), &FetchOptions::default())
        .await
        .expect("raw retained capture");
    assert_eq!(cached.body, SUMMARY);
    assert_eq!(cached.fetched_at, CAPTURED);
}

#[tokio::test]
async fn foreign_summary_never_attaches_coaches_aliases_or_address_and_remains_unfinished() {
    let body = changed_summary(|summary| {
        summary["id"] = "another-organization".into();
        summary["name"] = "Foreign School".into();
        summary["address"]["address1"] = "999 Foreign Road".into();
    });
    let run = FixtureRun::new(DIRECTORY, &body);
    let report = run.collect().await;
    assert_eq!((report.rows, report.errors), (0, 1));
    let school = run.school();
    assert_eq!(school.name, SCHOOL_NAME);
    assert_eq!(school.city.as_deref(), Some("Asheville"));
    assert_eq!(school.classification.as_deref(), Some("6A"));
    assert_eq!(school.aliases, Vec::<String>::new());
    assert_eq!(school.postal_addresses.len(), 1);
    assert_eq!(
        school.postal_addresses[0]
            .address()
            .line1()
            .map(|line| line.as_str()),
        Some("1 Rocket Drive")
    );
    assert_eq!(run.coaches(), Vec::<CanonicalCoach>::new());
    assert!(!run
        .store
        .journal_keys("coach_directories_schools_v3")
        .expect("completion journal")
        .contains("NC:ZCUM49"));
    seed(&run.fetcher, &summary_url("ZCUM49"), SUMMARY);
    let recovered = run.collect().await;
    assert_eq!((recovered.rows, recovered.errors), (1, 0));
    assert_eq!(run.school().postal_addresses.len(), 2);
    assert_eq!(run.coaches().len(), 16);
}

#[tokio::test]
async fn alternate_published_streets_are_not_ranked_or_discarded() {
    let body = changed_summary(|summary| {
        summary["address"]["address1"] = "2 Published Alternate Drive".into();
        summary["address"]["address2"] = "Suite 7".into();
        summary["address"]["zip"] = "02803-0042".into();
    });
    let run = FixtureRun::new(DIRECTORY, &body);
    assert_eq!(run.collect().await.errors, 0);
    let school = run.school();
    let addresses: Vec<_> = school
        .postal_addresses
        .iter()
        .map(|claim| claim.address().line1().map(|line| line.as_str()))
        .collect();
    assert_eq!(
        addresses,
        vec![Some("1 Rocket Drive"), Some("2 Published Alternate Drive")]
    );
    let claim = &school.postal_addresses[1];
    assert_eq!(
        claim.address().line2().map(|line| line.as_str()),
        Some("Suite 7")
    );
    assert_eq!(claim.address().zip().map(|zip| zip.code()), Some("02803"));
    assert_eq!(
        claim.address().zip().and_then(|zip| zip.plus4()),
        Some("0042")
    );
    assert_eq!(claim.capture_sha256(), content_digest(&body));
}
