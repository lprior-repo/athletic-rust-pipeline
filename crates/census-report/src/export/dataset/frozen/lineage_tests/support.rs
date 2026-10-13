pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
pub(super) const FIXTURE_ROOT: &str = "CENSUS_LINEAGE_FIXTURE_ROOT";

pub(super) fn fixture_dir() -> TestResult<tempfile::TempDir> {
    match std::env::var_os(FIXTURE_ROOT) {
        Some(root) => Ok(tempfile::Builder::new()
            .prefix("report-lineage-")
            .tempdir_in(root)?),
        None => Ok(tempfile::tempdir()?),
    }
}
