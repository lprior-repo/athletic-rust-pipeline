use census_crawl::CrawlError;

#[derive(Debug)]
pub enum SourceAccessFailure {
    Primary(CrawlError),
    Retention {
        primary: Box<CrawlError>,
        secondary: Box<CrawlError>,
    },
}

impl From<CrawlError> for SourceAccessFailure {
    fn from(error: CrawlError) -> Self {
        Self::Primary(error)
    }
}

impl std::error::Error for SourceAccessFailure {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.primary())
    }
}

impl SourceAccessFailure {
    pub fn retention(primary: CrawlError, secondary: CrawlError) -> Self {
        Self::Retention {
            primary: Box::new(primary),
            secondary: Box::new(secondary),
        }
    }

    pub fn primary(&self) -> &CrawlError {
        match self {
            Self::Primary(error) => error,
            Self::Retention { primary, .. } => primary.as_ref(),
        }
    }

    pub fn retryable(&self) -> bool {
        self.primary().retryable()
    }
}

impl std::fmt::Display for SourceAccessFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Primary(error) => std::fmt::Display::fmt(error, formatter),
            Self::Retention { primary, secondary } => {
                std::fmt::Display::fmt(&super::combined_failure(primary, secondary), formatter)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use census_crawl::net::FetchError;
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn causal_access_failure_retains_exact_primary_secondary_and_error_source() -> TestResult {
        let failure = SourceAccessFailure::retention(
            CrawlError::Fetch(FetchError::RateLimited {
                url: "https://wi.milesplit.com/teams".to_string(),
                retry_after_secs: Some(120),
            }),
            CrawlError::Resource {
                resource: "source access snapshot",
                requested: 4097,
                limit: 4096,
            },
        );
        check!(
            matches!(failure.primary(), CrawlError::Fetch(FetchError::RateLimited { url, retry_after_secs: Some(120) }) if url == "https://wi.milesplit.com/teams")
        );
        let source = std::error::Error::source(&failure).ok_or("missing primary error source")?;
        check!(eq; source.to_string(), failure.primary().to_string());
        let source = source
            .downcast_ref::<CrawlError>()
            .ok_or("error source is not the primary crawl error")?;
        check!(std::ptr::eq(source, failure.primary()));
        check!(matches!(
            &failure,
            SourceAccessFailure::Retention { secondary, .. }
                if matches!(secondary.as_ref(), CrawlError::Resource {
                    resource: "source access snapshot",
                    requested: 4097,
                    limit: 4096
                })
        ));
        check!(failure.retryable());
        Ok(())
    }

    #[test]
    fn primary_access_failure_preserves_its_original_display_and_retry_class() -> TestResult {
        let error = CrawlError::Fetch(FetchError::Http {
            status: 403,
            url: "https://wi.milesplit.com/teams".to_string(),
        });
        let expected = error.to_string();
        let failure = SourceAccessFailure::from(error);
        check!(eq; failure.to_string(), expected);
        check!(eq; failure.retryable(), false);
        let source = std::error::Error::source(&failure)
            .and_then(|source| source.downcast_ref::<CrawlError>())
            .ok_or("missing primary crawl error source")?;
        check!(std::ptr::eq(source, failure.primary()));
        Ok(())
    }

    #[test]
    fn causal_access_failure_bounds_diagnostics_without_truncating_typed_evidence() -> TestResult {
        let primary_text = "primary".repeat(2000);
        let secondary_text = "secondary".repeat(2000);
        let failure = SourceAccessFailure::retention(
            CrawlError::Invariant {
                detail: primary_text.clone(),
            },
            CrawlError::Invariant {
                detail: secondary_text.clone(),
            },
        );
        let display = failure.to_string();
        check!(display.len() < 9000);
        check!(display.contains("retaining access evidence also failed"));
        check!(matches!(&failure, SourceAccessFailure::Retention {
            primary,
            secondary,
        } if matches!(primary.as_ref(), CrawlError::Invariant { detail } if detail == &primary_text)
            && matches!(secondary.as_ref(),
            CrawlError::Invariant { detail } if detail == &secondary_text)));
        Ok(())
    }

    #[test]
    fn source_access_failure_stays_within_the_result_error_size_budget() -> TestResult {
        check!(std::mem::size_of::<SourceAccessFailure>() < 128);
        Ok(())
    }
}
