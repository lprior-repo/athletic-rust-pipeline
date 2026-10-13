#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use census_report::report::{CorruptionClass, ExportResourceError, ReportError};
use census_service::restate_services::{blocking, job_error, JobError};
use census_service::spawn::Spawner;
use std::path::PathBuf;
use std::sync::Arc;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn handler_text(error: &restate_sdk::prelude::HandlerError) -> String {
    let borrowed: &dyn std::error::Error = error.as_ref();
    borrowed.to_string()
}

fn refusals() -> [(ExportResourceError, &'static str); 3] {
    [
        (
            ExportResourceError::RowTooLarge { what: "frozen line", bytes: 1_048_577, max: 1_048_576 },
            "export-row-too-large: frozen line of 1048577 bytes exceeds the 1048576-byte ceiling",
        ),
        (
            ExportResourceError::BudgetExceeded { what: "frozen json depth", needed: 65, available: 64 },
            "export-budget-exceeded: frozen json depth needs 65 against an available 64",
        ),
        (
            ExportResourceError::NeedsPartition { what: "frozen inventory", bytes: 4_194_305, max: 4_194_304 },
            "export-needs-partition: frozen inventory of 4194305 bytes needs another partition beyond the 4194304-byte envelope",
        ),
    ]
}

#[test]
fn export_resource_public_contract_preserves_variants_fields_and_terminal_413() -> TestResult {
    for (source, expected) in refusals() {
        let job = JobError::from(ReportError::Resource(source));
        check!(
            matches!(&job, JobError::Resource { source: actual, message } if *actual == source && message == expected)
        );
        let handler = job_error(job);
        check!(eq; handler_text(&handler), format!("Terminal error [413]: {expected}"));
    }
    Ok(())
}

#[test]
fn export_resource_cleanup_keeps_primary_classification_and_secondary_failure() -> TestResult {
    let source = ExportResourceError::BudgetExceeded {
        what: "frozen json depth",
        needed: 65,
        available: 64,
    };
    let report = ReportError::Cleanup {
        operation: Box::new(ReportError::Cleanup {
            operation: Box::new(ReportError::Resource(source)),
            cleanup: Box::new(ReportError::Invariant {
                detail: "inner cleanup failed".into(),
            }),
        }),
        cleanup: Box::new(ReportError::Invariant {
            detail: "outer cleanup failed".into(),
        }),
    };
    check!(eq; report.resource(), Some(&source));
    let job = JobError::from(report);
    check!(
        matches!(&job, JobError::Resource { source: actual, message } if *actual == source
        && message.matches("export-budget-exceeded:").count() == 1
        && message.contains("inner cleanup failed") && message.contains("outer cleanup failed"))
    );
    let handler = job_error(job);
    check!(handler_text(&handler).starts_with("Terminal error [413]: publication failed:"));
    check!(eq; handler_text(&handler).matches("export-budget-exceeded:").count(), 1);
    check!(handler_text(&handler).contains("inner cleanup failed"));
    check!(handler_text(&handler).contains("outer cleanup failed"));
    Ok(())
}

#[test]
fn export_resource_secondary_cleanup_does_not_reclassify_a_nonresource_operation() -> TestResult {
    let report = ReportError::Cleanup {
        operation: Box::new(ReportError::Invariant {
            detail: "ordinary operation failed".into(),
        }),
        cleanup: Box::new(ReportError::Resource(ExportResourceError::BudgetExceeded {
            what: "cleanup bytes",
            needed: 2,
            available: 1,
        })),
    };
    check!(eq; report.resource(), None);
    let expected = report.to_string();
    check!(matches!(JobError::from(report), JobError::Terminal { message } if message == expected));
    let report = ReportError::ArtifactCorrupt {
        path: PathBuf::from("fixture.jsonl"),
        detail: "bad digest".into(),
    };
    check!(eq; report.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    let expected = report.to_string();
    check!(
        matches!(JobError::from(report), JobError::Transient { message } if message == expected)
    );
    Ok(())
}

#[tokio::test]
async fn export_resource_blocking_boundary_carries_typed_refusal_into_the_handler() -> TestResult {
    let region = Arc::new(Spawner::with_capacity(1));
    let result = blocking(region, || {
        Err::<(), _>(ReportError::Resource(ExportResourceError::RowTooLarge {
            what: "frozen line",
            bytes: 1_048_577,
            max: 1_048_576,
        }))
    })
    .await;
    let job = match result {
        Ok(()) => return Err("resource-refusing blocking job succeeded".into()),
        Err(job) => job,
    };
    check!(matches!(
        &job,
        JobError::Resource {
            source: ExportResourceError::RowTooLarge {
                what: "frozen line",
                bytes: 1_048_577,
                max: 1_048_576,
            },
            ..
        }
    ));
    check!(eq; handler_text(&job_error(job)), "Terminal error [413]: export-row-too-large: frozen line of 1048577 bytes exceeds the 1048576-byte ceiling");
    Ok(())
}

#[test]
fn export_resource_cli_context_preserves_one_stable_code() -> TestResult {
    for (source, expected) in refusals() {
        let report = ReportError::Resource(source);
        check!(eq; report.to_string(), expected);
        let contextual = anyhow::Error::new(report).context("publishing workbook");
        let rendered = format!("{contextual:#}");
        check!(rendered.contains(expected));
        check!(eq; rendered.matches(source.code()).count(), 1);
    }
    Ok(())
}

#[test]
fn journal_read_limits_remain_terminal_and_nonretryable_across_existing_consumers() -> TestResult {
    for through_report in [false, true] {
        let source = census_store::StoreError::JournalReadLimit {
            phase: "retained_capture".into(),
            key: "manifest".into(),
            what: "encoded envelope",
            needed: 65,
            max: 64,
        };
        let expected = source.to_string();
        let job = if through_report {
            JobError::from(ReportError::Store(source))
        } else {
            JobError::from(source)
        };
        check!(matches!(&job, JobError::Terminal { message } if message == &expected));
        check!(eq; handler_text(&job_error(job)), format!("Terminal error [500]: {expected}"));
    }
    let crawl = census_crawl::CrawlError::from(census_store::StoreError::JournalReadLimit {
        phase: "retained_capture".into(),
        key: "chunk".into(),
        what: "physical chunk",
        needed: 65,
        max: 64,
    });
    check!(!crawl.retryable());
    Ok(())
}
