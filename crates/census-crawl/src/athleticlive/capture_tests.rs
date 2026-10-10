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

fn producer_metadata(url: String, bytes: usize) -> CacheMeta {
    CacheMeta {
        redirects: Vec::new(),
        url,
        method: "GET".to_string(),
        status: 200,
        bytes,
        content_digest: "unused-by-the-admission-bound".to_string(),
        fetched_at: "2026-09-18T13:42:19Z".to_string(),
        ..CacheMeta::default()
    }
}

#[test]
fn capture_refuses_input_path_over_4096_bytes() -> TestResult {
    let (_root, store, fetcher) = scratch()?;
    let ctx = context(&store, &fetcher)?;
    let input = "x".repeat(4097);
    let metadata = producer_metadata("https://live.athletic.net/meets/1".to_string(), 0);
    let error = match freeze(&ctx, &input, Some(&metadata), 32) {
        Err(error) => error,
        Ok(_) => return Err("a 4097-byte capture input path reached the reader".into()),
    };
    match error {
        CrawlError::Resource {
            resource,
            requested,
            limit,
        } => {
            check!(eq; resource, "LIVE input path bytes");
            check!(eq; requested, 4097);
            check!(eq; limit, 4096);
        }
        other => return Err(format!("refused as {other:?}").into()),
    }
    Ok(())
}

#[test]
fn capture_refuses_producer_metadata_field_over_4096_bytes() -> TestResult {
    let (_root, store, fetcher) = scratch()?;
    let ctx = context(&store, &fetcher)?;
    let metadata = producer_metadata(format!("https://live.athletic.net/{}", "a".repeat(4096)), 0);
    let error = match freeze(&ctx, "absent-capture.json", Some(&metadata), 32) {
        Err(error) => error,
        Ok(_) => return Err("an over-limit metadata field reached the reader".into()),
    };
    match error {
        CrawlError::Resource {
            resource,
            requested,
            limit,
        } => {
            check!(eq; resource, "LIVE metadata field bytes");
            check!(eq; requested, 4097);
            check!(eq; limit, 4096);
        }
        other => return Err(format!("refused as {other:?}").into()),
    }
    Ok(())
}

#[test]
fn capture_refuses_declared_body_over_32_bytes() -> TestResult {
    let (_root, store, fetcher) = scratch()?;
    let ctx = context(&store, &fetcher)?;
    let metadata = producer_metadata("https://live.athletic.net/meets/1".to_string(), 33);
    let error = match freeze(&ctx, "absent-capture.json", Some(&metadata), 32) {
        Err(error) => error,
        Ok(_) => return Err("a declared 33-byte body crossed a 32-byte limit".into()),
    };
    match error {
        CrawlError::Resource {
            resource,
            requested,
            limit,
        } => {
            check!(eq; resource, "LIVE capture bytes");
            check!(eq; requested, 33);
            check!(eq; limit, 32);
        }
        other => return Err(format!("refused as {other:?}").into()),
    }
    Ok(())
}

#[test]
fn capture_refuses_physical_body_over_32_bytes_through_the_chunk_loop() -> TestResult {
    let (root, store, fetcher) = scratch()?;
    let ctx = context(&store, &fetcher)?;
    let input = root.path().join("over-limit.json");
    std::fs::File::create(&input)?.set_len(33)?;
    let path = input.to_string_lossy().into_owned();
    let metadata = producer_metadata("https://live.athletic.net/meets/1".to_string(), 32);
    let error = match freeze(&ctx, &path, Some(&metadata), 32) {
        Err(error) => error,
        Ok(_) => return Err("a 33-byte physical capture body was read".into()),
    };
    match error {
        CrawlError::Resource {
            resource,
            requested,
            limit,
        } => {
            check!(eq; resource, "LIVE capture bytes");
            check!(eq; requested, 33);
            check!(eq; limit, 32);
        }
        other => return Err(format!("refused as {other:?}").into()),
    }
    Ok(())
}
