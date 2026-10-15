use super::TestResult;
use census_crawl::convert::{read_file_capped, CONVERTER_MAX_OUTPUT_BYTES};
use census_crawl::CrawlError;

#[test]
fn converter_file_read_preserves_empty_exact_and_binary_contents_on_every_platform() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let path = scratch.path().join("bytes");
    std::fs::write(&path, [])?;
    check!(eq; read_file_capped(&path, 0)?, Vec::<u8>::new());
    check!(eq; read_file_capped(&path, CONVERTER_MAX_OUTPUT_BYTES)?, Vec::<u8>::new());
    let bytes = b"\0fixture\xff";
    std::fs::write(&path, bytes)?;
    check!(eq; read_file_capped(&path, bytes.len())?, bytes);
    let limit = bytes.len().checked_sub(1).ok_or("empty fixture")?;
    let outcome = read_file_capped(&path, limit);
    let Err(CrawlError::Resource {
        resource,
        requested,
        limit: held,
    }) = outcome
    else {
        return Err(format!("expected exact resource refusal: {outcome:?}").into());
    };
    check!(eq; resource, "converter output bytes");
    check!(eq; requested, bytes.len());
    check!(eq; held, limit);
    Ok(())
}

#[test]
fn converter_file_read_attributes_missing_file_without_platform_refusal() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let path = scratch.path().join("absent");
    let outcome = read_file_capped(&path, 1);
    let Err(CrawlError::Io { path: held, source }) = outcome else {
        return Err(format!("expected portable attributed I/O error: {outcome:?}").into());
    };
    check!(eq; held, path);
    check!(eq; source.kind(), std::io::ErrorKind::NotFound);
    Ok(())
}

#[cfg(unix)]
#[test]
fn converter_file_read_refuses_directory_as_nonregular_input() -> TestResult {
    let scratch = tempfile::tempdir()?;
    let outcome = read_file_capped(scratch.path(), 1);
    let Err(CrawlError::Io { path, source }) = outcome else {
        return Err(format!("expected nonregular-file refusal: {outcome:?}").into());
    };
    check!(eq; path, scratch.path());
    check!(eq; source.kind(), std::io::ErrorKind::InvalidInput);
    Ok(())
}
