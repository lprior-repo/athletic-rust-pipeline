use super::*;
use crate::net::{
    cache::{content_digest, write_cache, CacheMeta},
    Fetcher,
};
use census_store::{Store, Table};

fn require_send<F: std::future::Future + Send>(future: F) -> F {
    future
}

fn seed_cache(cache: &std::path::Path, url: &str, body: &str) -> TestResult {
    seed_cache_at(cache, url, body, "2026-09-27T12:00:00Z")
}

fn seed_cache_at(cache: &std::path::Path, url: &str, body: &str, stamp: &str) -> TestResult {
    let key = Fetcher::key_for("GET", url, "");
    write_cache(
        &cache.join(format!("{key}.body")),
        &cache.join(format!("{key}.meta.json")),
        body.as_bytes(),
        &CacheMeta {
            redirects: Vec::new(),
            url: url.to_owned(),
            method: "GET".to_owned(),
            status: 200,
            content_digest: content_digest(body.as_bytes()),
            bytes: body.len(),
            fetched_at: stamp.to_owned(),
            ..CacheMeta::default()
        },
    )?;
    Ok(())
}

#[test]
fn collect_stores_the_requested_school_and_its_coach_rows_from_the_cache() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let dir = tempfile::tempdir()?;
        let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
        let cache = dir.path().join("http");
        let fetcher = Fetcher::new(&cache, None, std::time::Duration::ZERO, std::collections::HashMap::new(), Vec::new())?.with_offline(true);
        seed_cache(&cache, DIRECTORY_URL, DIRECTORY)?;
        seed_cache(&cache, &school_page_url(&cherry_creek_slug()?), SCHOOL_PAGE)?;
        let store = Store::open(dir.path().join("store"))?;
        let ctx = context(&fetcher, &store, &capture)?;
        let report = require_send(collect(&ctx, &options())).await?;
        check!(eq; report.rows, 1);
        check!(eq; report.errors, 0);
        check!(eq; report.with_email, 0);
        check!(eq; report.requests, 0);
        check!(eq; report.from_cache, 2);
        let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
        check!(eq; schools.len(), 1);
        let school = schools.first().ok_or("requested school")?;
        check!(eq; school.name, "Cherry Creek");
        check!(eq; school.state, Some(UsJurisdiction::Colorado));
        check!(eq; school.association.as_deref(), Some("CHSAA"));
        check!(eq; school.city.as_deref(), Some("Greenwood Village"));
        let coaches = store.scan::<CanonicalCoach>(Table::Coaches)?;
        let expected = golden_rows(GOLDEN_COACHES)?.iter().filter(|row|
            matches!(field(row, "sport").as_deref(), Some("Track" | "CrossCountry")))
            .map(|row| (field(row, "person").map_or(Default::default(), core::convert::identity),
                field(row, "sport").map_or(Default::default(), core::convert::identity),
                field(row, "role").map_or(Default::default(), core::convert::identity),
                field(row, "gender").map_or(Default::default(), core::convert::identity)))
            .collect::<std::collections::BTreeSet<_>>();
        check!(eq; coaches.len(), 50);
        check!(eq; coaches.iter().map(coach_key).collect::<std::collections::BTreeSet<_>>(), expected);
        check!(coaches.iter().all(|coach| coach.school == school.id));
        Ok(())
    })
}

#[test]
fn a_journalled_school_is_skipped_on_the_next_run() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let dir = tempfile::tempdir()?;
        let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
        let cache = dir.path().join("http");
        let fetcher = Fetcher::new(&cache, None, std::time::Duration::ZERO, std::collections::HashMap::new(), Vec::new())?.with_offline(true);
        seed_cache(&cache, DIRECTORY_URL, DIRECTORY)?;
        seed_cache(&cache, &school_page_url(&cherry_creek_slug()?), SCHOOL_PAGE)?;
        let store = Store::open(dir.path().join("store"))?;
        let ctx = context(&fetcher, &store, &capture)?;
        collect(&ctx, &options()).await?;
        let before = [store.walk_table(Table::Schools)?, store.walk_table(Table::Coaches)?, store.walk_table(Table::SourceObservations)?];
        let coaches = store.scan::<CanonicalCoach>(Table::Coaches)?;
        let second = collect(&ctx, &options()).await?;
        check!(eq; second.rows, 0);
        check!(eq; second.errors, 0);
        check!(eq; second.requests, 0);
        check!(eq; [store.walk_table(Table::Schools)?, store.walk_table(Table::Coaches)?, store.walk_table(Table::SourceObservations)?], before);
        check!(eq; store.scan::<CanonicalCoach>(Table::Coaches)?, coaches);
        Ok(())
    })
}

fn context<'a>(
    fetcher: &'a Fetcher,
    store: &'a Store,
    capture: &'a crate::ResultsCaptureContext,
) -> TestResult<crate::AdapterContext<'a>> {
    Ok(crate::AdapterContext {
        capture: crate::CaptureContext::DurableV1(capture),
        fetcher,
        store,
        refresh: false,
        school_year: census_domain::model::SchoolYear::new(2026).ok_or("school year")?,
        observed_on: OBSERVED_ON.to_owned(),
        performance_as_of: chrono::NaiveDate::parse_from_str(OBSERVED_ON, "%Y-%m-%d")?,
        recording: None,
    })
}

fn options() -> Options {
    Options {
        states: vec![UsJurisdiction::Colorado],
        school_names: vec!["Cherry Creek".to_owned()],
        ..Options::default()
    }
}

#[test]
fn directory_school_dates_do_not_follow_detail_capture_refresh() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let dir = tempfile::tempdir()?;
        let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
        let cache = dir.path().join("http");
        let fetcher = Fetcher::new(&cache, None, std::time::Duration::ZERO, std::collections::HashMap::new(), Vec::new())?.with_offline(true);
        let directory_stamp = "2026-09-20T12:00:00Z";
        let url = school_page_url(&cherry_creek_slug()?);
        seed_cache_at(&cache, DIRECTORY_URL, DIRECTORY, directory_stamp)?;
        let store = Store::open(dir.path().join("store"))?;
        let mut ctx = context(&fetcher, &store, &capture)?;
        ctx.observed_on = "2031-10-12".to_string();
        let mut school_rows = None;
        let mut retained_stamps = Vec::new();
        for (detail_stamp, raw_rows) in [("2026-09-22T12:00:00Z", 50), ("2026-09-24T12:00:00Z", 100)] {
            retained_stamps.push(detail_stamp);
            seed_cache_at(&cache, &url, SCHOOL_PAGE, detail_stamp)?;
            let report = require_send(collect(&ctx, &options())).await?;
            check!(eq; report.errors, 0);
            check!(eq; report.requests, 0);
            let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
            let school = schools.first().ok_or("school")?;
            check!(school.evidence.iter().all(|evidence| evidence.observed_on == directory_stamp));
            check!(school.evidence.iter().all(|evidence| evidence.source.url.as_deref() == Some(DIRECTORY_URL)));
            let observations = store.scan::<census_domain::model::SourceObservation>(Table::SourceObservations)?;
            check!(!observations.is_empty());
            check!(observations.iter().all(|row| row.observed_on() == directory_stamp));
            let rows = [store.walk_table(Table::Schools)?, store.walk_table(Table::SourceObservations)?];
            if let Some(previous) = &school_rows {
                check!(eq; &rows, previous);
            }
            school_rows = Some(rows);
            let coaches = store.scan::<CanonicalCoach>(Table::Coaches)?;
            check!(eq; store.walk_table(Table::Coaches)?.rows, raw_rows);
            check!(eq; coaches.len(), 50);
            check!(coaches.iter().all(|coach| retained_stamps.iter().all(|stamp|
                coach.evidence.iter().any(|evidence| evidence.observed_on == *stamp
                    && evidence.source.url.as_deref() == Some(url.as_str())))));
        }
        let coaches = store.scan::<CanonicalCoach>(Table::Coaches)?;
        let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
        let observations = store.scan::<census_domain::model::SourceObservation>(Table::SourceObservations)?;
        let raw_coaches = store.walk_table(Table::Coaches)?;
        drop(store);
        let reopened = Store::open(dir.path().join("store"))?;
        check!(eq; reopened.walk_table(Table::Coaches)?, raw_coaches);
        check!(eq; reopened.scan::<CanonicalCoach>(Table::Coaches)?, coaches);
        check!(eq; reopened.scan::<CanonicalSchool>(Table::Schools)?, schools);
        check!(eq; reopened.scan::<census_domain::model::SourceObservation>(Table::SourceObservations)?, observations);
        Ok(())
    })
}
