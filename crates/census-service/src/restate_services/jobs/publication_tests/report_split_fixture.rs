use super::TestResult;

fn temporary() -> TestResult<tempfile::TempDir> {
    Ok(tempfile::tempdir_in(std::fs::canonicalize(
        std::env::temp_dir(),
    )?)?)
}

mod fixture;
mod publication;

pub(super) async fn service_split_acceptance() -> TestResult {
    publication::service_split_acceptance().await
}
