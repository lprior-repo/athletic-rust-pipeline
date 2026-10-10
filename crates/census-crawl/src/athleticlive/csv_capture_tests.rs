use super::*;
use crate::net::cache::content_digest;
use census_domain::model::{CanonicalMeet, SchoolYear};
use census_store::Store;

#[test]
fn physical_csv_capture_admits_published_past_and_retains_future_debt(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let fetcher = crate::net::Fetcher::new(
        store.http_cache_dir(),
        None,
        std::time::Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )?;
    let input = root.path().join("published.csv");
    let body = b"tenant,athleticlive_meet_id,name,state,start\ntimer,123,Published Meet,Illinois,2025-05-01\ntimer,124,Future Meet,Illinois,2027-05-01\n";
    std::fs::write(&input, body)?;
    let metadata = CacheMeta {
        redirects: Vec::new(),
        url: "https://live.athletic.net/meets.csv".to_string(),
        method: "GET".to_string(),
        status: 200,
        bytes: body.len(),
        content_digest: content_digest(body),
        fetched_at: "2026-09-18T13:42:19Z".to_string(),
        ..CacheMeta::default()
    };
    let options = Options::for_input(input.to_string_lossy(), metadata.clone());
    let ctx = AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::DEFAULT,
        observed_on: "2099-01-01".to_string(),
        performance_as_of: chrono::NaiveDate::parse_from_str("2026-10-07", "%Y-%m-%d")?,
        recording: None,
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let report = runtime.block_on(collect(&ctx, &options))?;
    let meets: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    check!(eq; meets.iter().map(|meet| meet.name.as_str()).collect::<Vec<_>>(), vec!["Published Meet"]);
    let meet = meets.first().ok_or("published meet is missing")?;
    let evidence = meet
        .evidence
        .first()
        .ok_or("physical capture evidence is missing")?;
    check!(eq; evidence.observed_on, "2026-09-18");
    check!(eq; evidence.source.url.as_deref(), Some(metadata.url.as_str()));
    check!(eq; evidence.note.as_deref(), Some(format!("capture sha256={}", metadata.content_digest).as_str()));
    check!(eq; report.disposition, CollectionDisposition::Partial);
    check!(eq; report.unfinished, vec![format!("{}#record=2", input.display())]);
    std::fs::write(&input, b"overwritten producer body")?;
    check!(runtime.block_on(collect(&ctx, &options)).is_err());
    let preserved: Vec<CanonicalMeet> = store.scan(Table::Meets)?;
    check!(eq; preserved, meets);
    Ok(())
}
