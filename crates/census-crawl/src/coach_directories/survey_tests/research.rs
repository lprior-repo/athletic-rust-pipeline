use super::{fixture, make_offline_fetcher, TestResult};
use crate::coach_directories::{collect, directory_page_url, summary_url, Options};
use crate::AdapterContext;
use census_domain::model::{
    school_contact_research, CanonicalCoach, CanonicalSchool, ContactResearch,
    ContactResearchOutcome as Outcome, ContactResearchSubject as Subject, SchoolYear,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

fn options() -> Options {
    Options {
        states: vec![UsJurisdiction::NorthCarolina],
        school_names: vec!["A.C. Reynolds High School".to_owned()],
        ..Options::default()
    }
}

async fn assess(
    root: &std::path::Path,
    summary: Option<&[u8]>,
    acquired_at: &str,
) -> TestResult<(Store, crate::AdapterReport)> {
    let store = Store::open(root.join("store"))?;
    let fetcher = make_offline_fetcher(root)?;
    let mut directory: serde_json::Value =
        serde_json::from_str(&fixture("coach_directories/nchsaa_directory_p1.json")?)?;
    directory["results"]
        .as_array_mut()
        .ok_or("published directory rows")?
        .retain(|row| row["shortCode"] == "ZCUM49");
    directory["totalPages"] = 1.into();
    directory["totalResults"] = 1.into();
    seed(
        &fetcher,
        &directory_page_url("NCHSAA", 1),
        &serde_json::to_vec(&directory)?,
        acquired_at,
    )?;
    if let Some(summary) = summary {
        seed(&fetcher, &summary_url("ZCUM49"), summary, acquired_at)?;
    }
    let ctx = AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("year")?,
        observed_on: "2026-10-07".to_owned(),
        performance_as_of: chrono::NaiveDate::parse_from_str("2026-10-07", "%Y-%m-%d")?,
        recording: None,
    };
    let report = collect(&ctx, &options()).await?;
    Ok((store, report))
}

fn retained(store: &Store) -> Result<CanonicalSchool, Box<dyn std::error::Error>> {
    store
        .scan::<CanonicalSchool>(Table::Schools)?
        .into_iter()
        .find(|school| school.name == "A.C. Reynolds High School")
        .ok_or_else(|| "durable zero-coach school".into())
}

fn assert_outcomes(school: &CanonicalSchool, outcome: Outcome) -> TestResult {
    for program in ContactResearch::programs() {
        check!(eq; school_contact_research(school, &program, SchoolYear::new(2026).ok_or("year")?), outcome);
        let row = school
            .contact_research
            .iter()
            .find(|row| row.subject == Subject::Program(program.clone()))
            .ok_or("program research record")?;
        let attempt = row.attempts.first().ok_or("attempt history")?;
        check!(eq; attempt.locator, summary_url("ZCUM49"));
    }
    Ok(())
}

fn zero_staff_summary() -> TestResult<String> {
    let mut summary: serde_json::Value =
        serde_json::from_str(&fixture("coach_directories/nc_staff_summary_zcum49.json")?)?;
    summary["staff"] = serde_json::json!([]);
    summary["teams"] = serde_json::json!([]);
    Ok(serde_json::to_string(&summary)?)
}

#[test]
fn cen16_owned_zero_staff_summary_records_completed_empty_without_coaches() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let summary = zero_staff_summary()?;
            let (store, report) =
                assess(dir.path(), Some(summary.as_bytes()), "2026-09-29T00:00:00Z").await?;
            check!(eq; report.errors, 0);
            check!(eq; store.scan::<CanonicalCoach>(Table::Coaches)?, vec![]);
            let digest = crate::net::cache::content_digest(summary.as_bytes());
            let school = retained(&store)?;
            assert_outcomes(&school, Outcome::CompletedEmpty)?;
            for attempt in school.contact_research.iter().flat_map(|row| &row.attempts) {
                check!(eq; attempt.acquired_at, "2026-09-29T00:00:00Z");
                check!(eq; attempt.source_sha256.as_deref(), Some(digest.as_str()));
            }
            Ok(())
        })
}

#[test]
fn cen16_offline_missing_summary_records_blocked_without_coaches() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let started = chrono::Utc::now().timestamp();
            let (store, report) = assess(dir.path(), None, "2026-09-29T00:00:00Z").await?;
            let ended = chrono::Utc::now().timestamp();
            check!(eq; report.errors, 1);
            check!(eq; store.scan::<CanonicalCoach>(Table::Coaches)?, vec![]);
            let school = retained(&store)?;
            assert_outcomes(&school, Outcome::Blocked)?;
            for attempt in school.contact_research.iter().flat_map(|row| &row.attempts) {
                let attempted =
                    chrono::DateTime::parse_from_rfc3339(&attempt.acquired_at)?.timestamp();
                check!(attempted >= started && attempted <= ended);
                check!(eq; attempt.source_sha256, None);
            }
            Ok(())
        })
}

#[test]
fn cen16_malformed_retained_summary_records_failed_without_coaches() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let (store, report) = assess(dir.path(), Some(b"{"), "2026-09-29T00:00:00Z").await?;
            check!(eq; report.errors, 1);
            check!(eq; store.scan::<CanonicalCoach>(Table::Coaches)?, vec![]);
            assert_outcomes(&retained(&store)?, Outcome::Failed)?;
            Ok(())
        })
}

fn seed(fetcher: &crate::net::Fetcher, url: &str, body: &[u8], acquired_at: &str) -> TestResult {
    let key = crate::net::Fetcher::key_for("GET", url, "");
    let (body_path, meta_path) = fetcher.cache_paths(&key);
    crate::net::cache::write_cache(
        &body_path,
        &meta_path,
        body,
        &crate::net::cache::CacheMeta {
            redirects: Vec::new(),
            url: url.to_owned(),
            response_url: None,
            method: "GET".to_owned(),
            status: 200,
            representation: crate::net::RepresentationHeaders::default(),
            content_digest: crate::net::cache::content_digest(body),
            bytes: body.len(),
            fetched_at: acquired_at.to_owned(),
            etag: None,
            last_modified: None,
            content_type: Some("application/json".to_owned()),
        },
    )?;
    Ok(())
}

#[test]
fn cen16_owned_zero_staff_prior_capture_stays_stale_and_unfinished() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let summary = zero_staff_summary()?;
            let (store, report) =
                assess(dir.path(), Some(summary.as_bytes()), "2025-09-29T00:00:00Z").await?;
            let school = retained(&store)?;
            assert_outcomes(&school, Outcome::Stale)?;
            check!(eq; report.disposition, crate::CollectionDisposition::Partial);
            for attempt in school.contact_research.iter().flat_map(|row| &row.attempts) {
                check!(eq; attempt.acquired_at, "2025-09-29T00:00:00Z");
                check!(attempt.outcome != Outcome::Exhausted);
            }
            Ok(())
        })
}
