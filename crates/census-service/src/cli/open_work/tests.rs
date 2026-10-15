use super::archive_growth::{
    compare_archive_activity, ArchiveArtifactGrowth, ArchiveArtifactGrowthOutcome,
    ArchiveGrowthReason,
};
use census_service::restate_services::{
    ArchiveCaptureActivity, ArchiveCaptureEpoch, ArchiveCaptureSample,
    ARCHIVE_ACTIVITY_METRIC_VERSION,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn sample(counts: (u64, u64, u64), started: u8, finished: u8) -> ArchiveCaptureActivity {
    ArchiveCaptureActivity::Measured(ArchiveCaptureSample {
        epoch: ArchiveCaptureEpoch {
            store_identity: "fixture-store".to_string(),
            archive_dev: Some(7),
            archive_ino: Some(11),
            metric_version: ARCHIVE_ACTIVITY_METRIC_VERSION,
        },
        started_at: format!("2026-10-15T00:00:{started:02}Z"),
        finished_at: format!("2026-10-15T00:00:{finished:02}Z"),
        capture_documents: counts.0,
        unique_body_files: counts.1,
        unique_body_payload_bytes: counts.2,
    })
}

fn measured(
    value: &mut ArchiveCaptureActivity,
) -> Result<&mut ArchiveCaptureSample, Box<dyn std::error::Error>> {
    match value {
        ArchiveCaptureActivity::Measured(sample) => Ok(sample),
        ArchiveCaptureActivity::Unmeasured(_) => Err("expected measured fixture".into()),
    }
}

#[test]
fn new_metadata_does_not_invent_new_bodies_or_bytes() -> TestResult {
    let before = sample((2, 1, 80), 0, 1);
    let after = sample((4, 1, 80), 10, 11);
    check!(eq; compare_archive_activity(&before, &after), ArchiveArtifactGrowthOutcome::Measured(ArchiveArtifactGrowth {
        elapsed_secs: 10,
        added_capture_documents: 2,
        added_body_files: 0,
        added_body_payload_bytes: 0,
    }));
    Ok(())
}

#[test]
fn idle_archive_is_measured_zero_not_unavailable() -> TestResult {
    let before = sample((2, 1, 80), 0, 1);
    let after = sample((2, 1, 80), 10, 11);
    check!(eq; compare_archive_activity(&before, &after), ArchiveArtifactGrowthOutcome::Measured(ArchiveArtifactGrowth {
        elapsed_secs: 10,
        added_capture_documents: 0,
        added_body_files: 0,
        added_body_payload_bytes: 0,
    }));
    check!(eq; compare_archive_activity(&before, &ArchiveCaptureActivity::default()), ArchiveArtifactGrowthOutcome::Unmeasured {
        reason: ArchiveGrowthReason::UnmeasuredSample,
    });
    Ok(())
}

#[test]
fn replacement_and_missing_archive_identity_cannot_produce_a_rate() -> TestResult {
    let before = sample((2, 1, 80), 0, 1);
    let mut candidates = [
        sample((4, 2, 160), 10, 11),
        sample((4, 2, 160), 10, 11),
        sample((4, 2, 160), 10, 11),
        sample((4, 2, 160), 10, 11),
    ];
    let [foreign_store, foreign_device, foreign_inode, no_identity] = &mut candidates;
    measured(foreign_store)?.epoch.store_identity = "other-store".to_string();
    measured(foreign_device)?.epoch.archive_dev = Some(8);
    measured(foreign_inode)?.epoch.archive_ino = Some(12);
    measured(no_identity)?.epoch.archive_ino = None;
    for after in &candidates {
        check!(eq; compare_archive_activity(&before, after), ArchiveArtifactGrowthOutcome::Unmeasured {
            reason: ArchiveGrowthReason::EpochChanged,
        });
    }
    Ok(())
}

#[test]
fn unknown_metric_contract_is_not_interpreted_as_current_growth() -> TestResult {
    let mut before = sample((2, 1, 80), 0, 1);
    let mut after = sample((4, 2, 160), 10, 11);
    measured(&mut before)?.epoch.metric_version = u32::MAX;
    measured(&mut after)?.epoch.metric_version = u32::MAX;
    check!(eq; compare_archive_activity(&before, &after), ArchiveArtifactGrowthOutcome::Unmeasured {
        reason: ArchiveGrowthReason::UnsupportedMetric,
    });
    Ok(())
}

#[test]
fn each_counter_regression_refuses_even_when_other_counters_grow() -> TestResult {
    let before = sample((2, 2, 80), 0, 1);
    for counts in [(1, 3, 160), (4, 1, 160), (4, 3, 79)] {
        check!(eq; compare_archive_activity(&before, &sample(counts, 10, 11)), ArchiveArtifactGrowthOutcome::Unmeasured {
            reason: ArchiveGrowthReason::CountRegression,
        });
    }
    Ok(())
}

#[test]
fn reversed_overlapping_and_zero_duration_samples_never_make_a_rate() -> TestResult {
    let before = sample((2, 1, 80), 0, 1);
    for (started, finished) in [(0, 11), (10, 9), (1, 1)] {
        check!(eq; compare_archive_activity(&before, &sample((4, 2, 160), started, finished)), ArchiveArtifactGrowthOutcome::Unmeasured {
            reason: ArchiveGrowthReason::TimeRegression,
        });
    }
    let mut invalid = sample((4, 2, 160), 10, 11);
    measured(&mut invalid)?.finished_at = "not a timestamp".to_string();
    check!(eq; compare_archive_activity(&before, &invalid), ArchiveArtifactGrowthOutcome::Unmeasured {
        reason: ArchiveGrowthReason::TimeRegression,
    });
    Ok(())
}

#[test]
fn given_out_of_range_capture_sample_when_parsing_then_validation_refuses() -> TestResult {
    use clap::Parser;
    for seconds in ["0", "61"] {
        let flag = format!("--capture-sample-seconds={seconds}");
        let error = crate::cli::Cli::try_parse_from(["census-service", "open-work", &flag])
            .err()
            .ok_or("out-of-range archive sampling interval admitted")?;
        check!(eq; error.kind(), clap::error::ErrorKind::ValueValidation);
    }
    Ok(())
}

#[test]
fn given_capture_sample_boundaries_when_parsing_then_interval_is_retained() -> TestResult {
    use clap::Parser;
    for (seconds, expected) in [("1", 1_u64), ("60", 60)] {
        let flag = format!("--capture-sample-seconds={seconds}");
        let cli = crate::cli::Cli::try_parse_from(["census-service", "open-work", &flag])?;
        let crate::cli::command::Command::OpenWork(args) = cli.command else {
            return Err("archive sampling command was not selected".into());
        };
        check!(eq; args.capture_sample_seconds, Some(expected));
    }
    Ok(())
}
