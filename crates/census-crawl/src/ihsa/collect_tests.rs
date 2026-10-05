use super::parse::parse_staff;
use super::{collect, Options, IHSA_API};
use crate::net::cache::{write_cache, CacheMeta};
use crate::net::Fetcher;
use crate::AdapterContext;
use census_domain::model::{AccessBlockKind, SchoolYear};
use census_store::Store;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::time::Duration;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SCHOOLS_URL: &str = "https://api.ihsa.org/v1/schools";
const OBSERVED_ON: &str = "2026-10-04";
const REVEAL_BODY: &str = r#"{"email":"office@example.org"}"#;

const FIXTURE_SCHOOLS: &str = include_str!("../../tests/fixtures/ihsa/v1_schools.json");
const FIXTURE_STAFF_OFFICE: &str =
    include_str!("../../tests/fixtures/ihsa/staff2_office_only.json");

struct FixtureRun {
    _root: tempfile::TempDir,
    store: Store,
    fetcher: Fetcher,
}

impl FixtureRun {
    fn new() -> TestResult<Self> {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path().join("store"))?;
        let fetcher = Fetcher::new(
            root.path().join("http"),
            None,
            Duration::from_millis(1),
            HashMap::new(),
            Vec::new(),
        )?
        .with_offline(true);
        let run = Self {
            _root: root,
            store,
            fetcher,
        };
        run.seed(SCHOOLS_URL, FIXTURE_SCHOOLS)?;
        Ok(run)
    }

    fn seed(&self, url: &str, body: &str) -> TestResult {
        let key = Fetcher::key_for("GET", url, "");
        let (body_path, meta_path) = self.fetcher.cache_paths(&key);
        write_cache(
            &body_path,
            &meta_path,
            body.as_bytes(),
            &CacheMeta {
                url: url.to_string(),
                response_url: Some(url.to_string()),
                method: "GET".to_string(),
                status: 200,
                content_digest: format!("{:x}", Sha256::digest(body.as_bytes())),
                bytes: body.len(),
                fetched_at: "2026-10-04T00:00:00Z".to_string(),
                etag: None,
                last_modified: None,
                content_type: Some("application/json".to_string()),
            },
        )?;
        Ok(())
    }

    fn serve_school(&self, school: &str) -> TestResult {
        self.seed(
            &format!("{IHSA_API}/v1/schools/{school}/staff2"),
            FIXTURE_STAFF_OFFICE,
        )?;
        self.serve_reveals(school)
    }

    fn serve_reveals(&self, school: &str) -> TestResult {
        for person in parse_staff(FIXTURE_STAFF_OFFICE)?
            .into_iter()
            .filter(|person| person.has_email == Some(true))
        {
            self.seed(
                &format!(
                    "{IHSA_API}/v1/schools/{school}/staff/{}/email",
                    person.person_id
                ),
                REVEAL_BODY,
            )?;
        }
        Ok(())
    }

    async fn run(&self) -> TestResult<crate::AdapterReport> {
        let context = AdapterContext {
            fetcher: &self.fetcher,
            store: &self.store,
            refresh: false,
            school_year: SchoolYear::new(2026).ok_or("2026 is a season")?,
            observed_on: OBSERVED_ON.to_string(),
            recording: None,
        };
        let report = collect(
            &context,
            &Options {
                observed_on: OBSERVED_ON.to_string(),
                ..Options::default()
            },
        )
        .await?;
        Ok(report)
    }

    fn journalled(&self) -> TestResult<HashSet<String>> {
        Ok(self.store.journal_keys("ihsa_schools")?)
    }
}

#[test]
fn a_school_whose_staff_fetch_never_reached_the_source_stays_open() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new()?;
            run.serve_school("0101")?;

            let first = run.run().await?;
            let notes = first.notes.join(" | ");
            check!(eq; first.rows, 1,
                "only the school with a reachable staff page completes; notes: {notes}");
            check!(eq; run.journalled()?, HashSet::from([String::from("IL:0101")]),
                "the schools the fetcher never reached must not be journalled as done");
            check!(
                first.notes.iter().any(|note| note.contains("left open")),
                "the report names every school left open: {:?}",
                first.notes
            );

            run.serve_school("0102")?;
            let second = run.run().await?;
            check!(eq; second.rows, 1, "the retry completes the previously unreachable school");
            check!(eq;
                run.journalled()?,
                HashSet::from([String::from("IL:0101"), String::from("IL:0102")])
            );

            let third = run.run().await?;
            check!(eq; third.rows, 0, "both reachable schools were already journalled");
            check!(eq; third.requests, 0, "nothing but cached bodies is read");

            Ok(())
        })
}

#[test]
fn a_school_whose_email_reveal_never_reached_the_source_stays_open() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new()?;
            run.seed(
                &format!("{IHSA_API}/v1/schools/0101/staff2"),
                FIXTURE_STAFF_OFFICE,
            )?;

            let first = run.run().await?;
            let notes = first.notes.join(" | ");
            check!(eq; first.rows, 0,
                "a school with an unrevealed email address must not complete; notes: {notes}");
            check!(eq; run.journalled()?, HashSet::new(),
                "no coach row is journalled from a half-revealed staff page");

            run.serve_reveals("0101")?;
            let second = run.run().await?;
            check!(eq; second.rows, 1, "the retry finishes the school");
            check!(eq; run.journalled()?, HashSet::from([String::from("IL:0101")]));

            Ok(())
        })
}

#[test]
fn a_recorded_host_cooldown_leaves_the_walk_open() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let run = FixtureRun::new()?;
            run.serve_school("0101")?;
            let _ = run
                .fetcher
                .record_access_condition(
                    "api.ihsa.org",
                    AccessBlockKind::RateLimited,
                    429,
                    Some(3600),
                    "test: api.ihsa.org answered 429",
                )
                .await;

            let report = run.run().await?;
            check!(eq; report.rows, 0, "no school completes inside a recorded cooldown");
            check!(eq; run.journalled()?, HashSet::new(),
                "cooldown-blocked schools must stay open for the next run");
            check!(
                report.notes.iter().any(|note| note.contains("cooldown")),
                "the report names the cooldown: {:?}",
                report.notes
            );
            Ok(())
        })
}
