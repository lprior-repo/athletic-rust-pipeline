use census_crawl::milesplit::{
    CaptureWitnessError, RetainedMetadataError, RetainedResultError, OWNED_CAPTURE_PHASE,
};
use census_store::{StoreError, StoreResult};

pub(super) fn source_unavailable(error: RetainedMetadataError) -> StoreResult<()> {
    match error {
        RetainedMetadataError::Store(source)
        | RetainedMetadataError::OwnedResult(RetainedResultError::Store(source)) => Err(source),
        RetainedMetadataError::OwnedResult(RetainedResultError::Capture(
            CaptureWitnessError::Store { source, .. },
        )) => Err(*source),
        RetainedMetadataError::OwnedResult(RetainedResultError::Capture(
            CaptureWitnessError::Budget {
                key,
                what,
                needed,
                max,
            },
        )) if matches!(
            what,
            "physical root"
                | "encoded envelope"
                | "physical chunk"
                | "chunk base64"
                | "chunk decoded"
                | "recorded successful read budget"
        ) =>
        {
            Err(StoreError::JournalReadLimit {
                phase: OWNED_CAPTURE_PHASE.into(),
                key,
                what,
                needed,
                max,
            })
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn recorder_budget() -> StoreError {
        StoreError::BudgetExceeded {
            what: "retained journal entries",
            needed: 40_001,
            available: 40_000,
        }
    }

    fn capture(error: CaptureWitnessError) -> RetainedMetadataError {
        RetainedMetadataError::OwnedResult(RetainedResultError::Capture(error))
    }

    #[test]
    fn every_store_wrapper_preserves_the_original_recorder_error() -> TestResult {
        for error in [
            RetainedMetadataError::Store(recorder_budget()),
            RetainedMetadataError::OwnedResult(RetainedResultError::Store(recorder_budget())),
            capture(CaptureWitnessError::Store {
                key: "acquisition/test/manifest".into(),
                source: Box::new(recorder_budget()),
            }),
        ] {
            check!(matches!(
                source_unavailable(error),
                Err(StoreError::BudgetExceeded {
                    what: "retained journal entries",
                    needed: 40_001,
                    available: 40_000,
                })
            ));
        }
        Ok(())
    }

    #[test]
    fn owned_journal_budget_fields_are_reconstructed_exactly() -> TestResult {
        for what in [
            "physical root",
            "encoded envelope",
            "physical chunk",
            "chunk base64",
            "chunk decoded",
            "recorded successful read budget",
        ] {
            let error = capture(CaptureWitnessError::Budget {
                key: "acquisition/test/manifest".into(),
                what,
                needed: 20_000,
                max: 16_384,
            });
            check!(matches!(
                source_unavailable(error),
                Err(StoreError::JournalReadLimit {
                    phase,
                    key,
                    what: actual,
                    needed: 20_000,
                    max: 16_384,
                }) if phase == OWNED_CAPTURE_PHASE
                    && key == "acquisition/test/manifest"
                    && actual == what
            ));
        }
        Ok(())
    }

    #[test]
    fn source_body_limits_and_documented_source_failures_remain_unavailable() -> TestResult {
        for what in ["body bytes", "body ceiling", "unknown source budget"] {
            check!(source_unavailable(capture(CaptureWitnessError::Budget {
                key: "acquisition/test/manifest".into(),
                what,
                needed: 2,
                max: 1,
            }))
            .is_ok());
        }
        for error in [
            RetainedMetadataError::Missing {
                key: "metadata/test/manifest".into(),
            },
            RetainedMetadataError::BodyBudget,
            RetainedMetadataError::SessionBudget,
            capture(CaptureWitnessError::Corrupt {
                key: "acquisition/test/manifest".into(),
                detail: "retained body digest differs".into(),
            }),
        ] {
            check!(source_unavailable(error).is_ok());
        }
        Ok(())
    }
}
