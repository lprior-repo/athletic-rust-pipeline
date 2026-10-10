use super::*;
use census_domain::model::SchoolYear;
use census_store::Store;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn scratch() -> TestResult<(tempfile::TempDir, Store, crate::net::Fetcher)> {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let fetcher = crate::net::Fetcher::new(
        store.http_cache_dir(),
        None,
        std::time::Duration::from_millis(1),
        std::collections::HashMap::new(),
        Vec::new(),
    )?;
    Ok((root, store, fetcher))
}

fn context<'a>(
    store: &'a Store,
    fetcher: &'a crate::net::Fetcher,
) -> TestResult<AdapterContext<'a>> {
    Ok(AdapterContext {
        fetcher,
        store,
        refresh: false,
        school_year: SchoolYear::DEFAULT,
        observed_on: "2099-01-01".to_string(),
        performance_as_of: chrono::NaiveDate::parse_from_str("2026-10-07", "%Y-%m-%d")?,
        recording: None,
    })
}

fn producer_metadata(bytes: usize) -> CacheMeta {
    CacheMeta {
        redirects: Vec::new(),
        url: "https://live.athletic.net/api/meets/1/event/2150205".to_string(),
        method: "GET".to_string(),
        status: 200,
        bytes,
        content_digest: "unused-by-the-admission-bound".to_string(),
        fetched_at: "2026-09-18T13:42:19Z".to_string(),
        ..CacheMeta::default()
    }
}

#[test]
fn result_capture_refuses_declared_body_over_1_mib() -> TestResult {
    let (root, store, fetcher) = scratch()?;
    let ctx = context(&store, &fetcher)?;
    let input = root.path().join("event-doc.json");
    std::fs::File::create(&input)?.set_len(u64::try_from(MAX_CAPTURE_BYTES + 1)?)?;
    let path = input.to_string_lossy().into_owned();
    let metadata = producer_metadata(MAX_CAPTURE_BYTES + 1);
    let error = match read(&ctx, &path, Some(&metadata)) {
        Err(error) => error,
        Ok(_) => return Err("a declared 1 MiB + 1 result capture was admitted".into()),
    };
    match error {
        CrawlError::Resource {
            resource,
            requested,
            limit,
        } => {
            check!(eq; resource, "LIVE capture bytes");
            check!(eq; requested, 1_048_577);
            check!(eq; limit, 1_048_576);
        }
        other => return Err(format!("refused as {other:?}").into()),
    }
    Ok(())
}

#[test]
fn result_capture_refuses_physical_body_over_1_mib() -> TestResult {
    let (root, store, fetcher) = scratch()?;
    let ctx = context(&store, &fetcher)?;
    let input = root.path().join("event-doc.json");
    std::fs::File::create(&input)?.set_len(u64::try_from(MAX_CAPTURE_BYTES + 1)?)?;
    let path = input.to_string_lossy().into_owned();
    let metadata = producer_metadata(MAX_CAPTURE_BYTES);
    let error = match read(&ctx, &path, Some(&metadata)) {
        Err(error) => error,
        Ok(_) => return Err("a 1 MiB + 1 physical result capture was read".into()),
    };
    match error {
        CrawlError::Resource {
            resource,
            requested,
            limit,
        } => {
            check!(eq; resource, "LIVE capture bytes");
            check!(eq; requested, 1_048_577);
            check!(eq; limit, 1_048_576);
        }
        other => return Err(format!("refused as {other:?}").into()),
    }
    Ok(())
}
