use super::FetchError;
use crate::CrawlError;
use census_store::{CaptureError, StoreError};
use std::error::Error;

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn capture_store_indirection_keeps_fetch_and_crawl_errors_below_128_bytes() -> TestResult {
    check!(std::mem::size_of::<FetchError>() < 128);
    check!(std::mem::size_of::<CrawlError>() < 128);
    Ok(())
}

#[test]
fn boxed_capture_store_conversions_preserve_the_exact_typed_refusal() -> TestResult {
    let expected = CaptureError::CacheRootMismatch.to_string();
    let errors = [
        FetchError::from(StoreError::from(CaptureError::CacheRootMismatch)),
        FetchError::from(CaptureError::CacheRootMismatch),
    ];
    for error in errors {
        check!(eq; error.to_string(), expected);
        check!(!error.retryable());
        check!(matches!(&error, FetchError::CaptureStore(store)
            if matches!(store.as_ref(), StoreError::Capture(CaptureError::CacheRootMismatch))));
        let wrapped = CrawlError::from(error);
        check!(eq; wrapped.to_string(), expected);
        check!(!wrapped.retryable());
        check!(
            matches!(wrapped, CrawlError::Fetch(FetchError::CaptureStore(store))
            if matches!(store.as_ref(), StoreError::Capture(CaptureError::CacheRootMismatch)))
        );
    }
    Ok(())
}

#[test]
fn boxed_capture_store_io_preserves_path_display_and_typed_source() -> TestResult {
    let path = std::path::PathBuf::from("capture-fixture/archive/body");
    let store = StoreError::Io {
        path: path.clone(),
        source: std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "capture fixture denied",
        ),
    };
    let expected = store.to_string();
    let fetch = FetchError::from(store);
    check!(eq; fetch.to_string(), expected);
    let source = fetch
        .source()
        .ok_or("missing fetch source")?
        .downcast_ref::<std::io::Error>()
        .ok_or("fetch source type changed")?;
    check!(eq; source.kind(), std::io::ErrorKind::PermissionDenied);
    check!(eq; source.to_string(), "capture fixture denied");
    check!(matches!(&fetch, FetchError::CaptureStore(store)
        if matches!(store.as_ref(), StoreError::Io { path: actual, .. } if actual == &path)));
    let crawl = CrawlError::from(fetch);
    check!(eq; crawl.to_string(), expected);
    let source = crawl
        .source()
        .ok_or("missing crawl source")?
        .downcast_ref::<std::io::Error>()
        .ok_or("crawl source type changed")?;
    check!(eq; source.kind(), std::io::ErrorKind::PermissionDenied);
    check!(eq; source.to_string(), "capture fixture denied");
    Ok(())
}
