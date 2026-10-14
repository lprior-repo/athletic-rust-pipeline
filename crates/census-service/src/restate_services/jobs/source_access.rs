use crate::census::SourceAccessFailure;
use crate::restate_services::JobError;

impl From<SourceAccessFailure> for JobError {
    fn from(failure: SourceAccessFailure) -> Self {
        let (primary, message) = match failure {
            SourceAccessFailure::Primary(primary) => return super::collect_error(primary),
            SourceAccessFailure::Retention { primary, secondary } => {
                let message =
                    crate::census::access::combined_failure(&primary, &secondary).to_string();
                (super::collect_error(*primary), message)
            }
        };
        match primary {
            JobError::Transient { .. } => JobError::Transient { message },
            JobError::Terminal { .. } => JobError::Terminal { message },
            JobError::Resource { source, .. } => JobError::Resource { source, message },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_crawl::{net::FetchError, CrawlError};
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn failure(primary: CrawlError) -> SourceAccessFailure {
        SourceAccessFailure::retention(
            primary,
            CrawlError::Resource {
                resource: "source access snapshot",
                requested: 4097,
                limit: 4096,
            },
        )
    }

    #[test]
    fn access_retention_failure_preserves_the_primary_job_classification() -> TestResult {
        for (status, retryable) in [(429, true), (503, true), (403, false), (404, false)] {
            let primary = CrawlError::Fetch(FetchError::Http {
                status,
                url: "https://wi.milesplit.com/teams".to_string(),
            });
            let expected = crate::census::access::combined_failure(
                &primary,
                &CrawlError::Resource {
                    resource: "source access snapshot",
                    requested: 4097,
                    limit: 4096,
                },
            )
            .to_string();
            match (JobError::from(failure(primary)), retryable) {
                (JobError::Transient { message }, true)
                | (JobError::Terminal { message }, false) => {
                    check!(eq; message, expected);
                }
                (outcome, _) => {
                    return Err(
                        format!("primary {status} changed classification: {outcome:?}").into(),
                    )
                }
            }
        }
        Ok(())
    }

    #[test]
    fn access_retention_failure_keeps_the_store_primary_retry_classification() -> TestResult {
        let primary = census_store::StoreError::Io {
            path: std::path::PathBuf::from("original-index"),
            source: std::io::Error::new(std::io::ErrorKind::WouldBlock, "original store failure"),
        };
        let result = JobError::from(failure(primary.into()));
        check!(
            matches!(result, JobError::Transient { ref message } if message.contains("original store failure") && message.contains("source access snapshot"))
        );
        Ok(())
    }
}
