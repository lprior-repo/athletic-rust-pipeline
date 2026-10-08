use super::*;
use crate::net::{
    cache::{content_digest, write_cache, CacheMeta},
    Fetcher,
};
use census_store::{Store, Table};

fn seed_cache(cache: &std::path::Path, url: &str, body: &str) -> TestResult {
    let key = Fetcher::key_for("GET", url, "");
    write_cache(
        &cache.join(format!("{key}.body")),
        &cache.join(format!("{key}.meta.json")),
        body.as_bytes(),
        &CacheMeta {
            url: url.to_owned(),
            method: "GET".to_owned(),
            status: 200,
            content_digest: content_digest(body.as_bytes()),
            bytes: body.len(),
            fetched_at: "2026-09-27T12:00:00Z".to_owned(),
            ..CacheMeta::default()
        },
    )?;
    Ok(())
}

#[test]
fn collect_stores_the_requested_school_and_its_coach_rows_from_the_cache() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let dir = tempfile::tempdir()?;
        let cache = dir.path().join("http");
        let fetcher = Fetcher::new(&cache, None, std::time::Duration::ZERO, std::collections::HashMap::new(), Vec::new())?.with_offline(true);
        seed_cache(&cache, DIRECTORY_URL, DIRECTORY)?;
        seed_cache(&cache, &school_page_url(&cherry_creek_slug()?), SCHOOL_PAGE)?;
        let store = Store::open(dir.path().join("store"))?;
        let ctx = context(&fetcher, &store)?;
        let report = collect(&ctx, &options()).await?;
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
            .map(|row| (field(row, "person").map_or_else(String::new, core::convert::identity),
                field(row, "sport").map_or_else(String::new, core::convert::identity),
                field(row, "role").map_or_else(String::new, core::convert::identity),
                field(row, "gender").map_or_else(String::new, core::convert::identity)))
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
        let cache = dir.path().join("http");
        let fetcher = Fetcher::new(&cache, None, std::time::Duration::ZERO, std::collections::HashMap::new(), Vec::new())?.with_offline(true);
        seed_cache(&cache, DIRECTORY_URL, DIRECTORY)?;
        seed_cache(&cache, &school_page_url(&cherry_creek_slug()?), SCHOOL_PAGE)?;
        let store = Store::open(dir.path().join("store"))?;
        let ctx = context(&fetcher, &store)?;
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

fn context<'a>(fetcher: &'a Fetcher, store: &'a Store) -> TestResult<crate::AdapterContext<'a>> {
    Ok(crate::AdapterContext {
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
