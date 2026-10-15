use anyhow::Result;
use census_service::restate_services::{
    ArchiveCaptureActivity, ArchiveCaptureEpoch, ArchiveCaptureSample, OpenWorkReply,
    ARCHIVE_ACTIVITY_METRIC_VERSION,
};
use std::io::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum ArchiveGrowthReason {
    UnmeasuredSample,
    EpochChanged,
    TimeRegression,
    CountRegression,
    UnsupportedMetric,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) struct ArchiveArtifactGrowth {
    pub elapsed_secs: u64,
    pub added_capture_documents: u64,
    pub added_body_files: u64,
    pub added_body_payload_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(super) enum ArchiveArtifactGrowthOutcome {
    Measured(ArchiveArtifactGrowth),
    Unmeasured { reason: ArchiveGrowthReason },
}

pub(super) fn compare_archive_activity(
    first: &ArchiveCaptureActivity,
    second: &ArchiveCaptureActivity,
) -> ArchiveArtifactGrowthOutcome {
    let (before, after) = match (first, second) {
        (ArchiveCaptureActivity::Measured(before), ArchiveCaptureActivity::Measured(after)) => {
            (before, after)
        }
        _ => {
            return ArchiveArtifactGrowthOutcome::Unmeasured {
                reason: ArchiveGrowthReason::UnmeasuredSample,
            }
        }
    };
    match compare_measured_samples(before, after) {
        Ok(growth) => ArchiveArtifactGrowthOutcome::Measured(growth),
        Err(reason) => ArchiveArtifactGrowthOutcome::Unmeasured { reason },
    }
}

fn compare_measured_samples(
    before: &ArchiveCaptureSample,
    after: &ArchiveCaptureSample,
) -> std::result::Result<ArchiveArtifactGrowth, ArchiveGrowthReason> {
    if before.epoch.metric_version != ARCHIVE_ACTIVITY_METRIC_VERSION
        || after.epoch.metric_version != ARCHIVE_ACTIVITY_METRIC_VERSION
    {
        return Err(ArchiveGrowthReason::UnsupportedMetric);
    }
    if !epochs_match(&before.epoch, &after.epoch) {
        return Err(ArchiveGrowthReason::EpochChanged);
    }
    let elapsed = archive_elapsed_secs(before, after)
        .filter(|elapsed| *elapsed > 0)
        .ok_or(ArchiveGrowthReason::TimeRegression)?;
    Ok(ArchiveArtifactGrowth {
        elapsed_secs: elapsed,
        added_capture_documents: after
            .capture_documents
            .checked_sub(before.capture_documents)
            .ok_or(ArchiveGrowthReason::CountRegression)?,
        added_body_files: after
            .unique_body_files
            .checked_sub(before.unique_body_files)
            .ok_or(ArchiveGrowthReason::CountRegression)?,
        added_body_payload_bytes: after
            .unique_body_payload_bytes
            .checked_sub(before.unique_body_payload_bytes)
            .ok_or(ArchiveGrowthReason::CountRegression)?,
    })
}

fn epochs_match(before: &ArchiveCaptureEpoch, after: &ArchiveCaptureEpoch) -> bool {
    before.store_identity == after.store_identity
        && before.metric_version == after.metric_version
        && matches!((before.archive_dev, after.archive_dev), (Some(left), Some(right)) if left == right)
        && matches!((before.archive_ino, after.archive_ino), (Some(left), Some(right)) if left == right)
}

fn archive_elapsed_secs(
    before: &ArchiveCaptureSample,
    after: &ArchiveCaptureSample,
) -> Option<u64> {
    let first_start = chrono::DateTime::parse_from_rfc3339(&before.started_at).ok()?;
    let first = chrono::DateTime::parse_from_rfc3339(&before.finished_at).ok()?;
    let second_start = chrono::DateTime::parse_from_rfc3339(&after.started_at).ok()?;
    let second = chrono::DateTime::parse_from_rfc3339(&after.finished_at).ok()?;
    if first < first_start || second_start < first || second < second_start {
        return None;
    }
    let delta = second.timestamp().checked_sub(first.timestamp())?;
    u64::try_from(delta).ok()
}

pub(super) fn print_archive_growth(growth: &ArchiveArtifactGrowthOutcome) -> Result<()> {
    let mut output = std::io::stdout().lock();
    match growth {
        ArchiveArtifactGrowthOutcome::Measured(measured) => {
            writeln!(
                output,
                "archive artifact growth: +{} documents, +{} bodies (+{} bytes) in {}s",
                measured.added_capture_documents,
                measured.added_body_files,
                measured.added_body_payload_bytes,
                measured.elapsed_secs
            )?;
        }
        ArchiveArtifactGrowthOutcome::Unmeasured { reason } => {
            writeln!(output, "archive artifact growth: unmeasured ({reason:?})")?;
        }
    }
    Ok(())
}

pub(super) fn print_sample_json(
    reply: &OpenWorkReply,
    growth: &ArchiveArtifactGrowthOutcome,
) -> Result<()> {
    #[derive(serde::Serialize)]
    struct Sample<'a> {
        open_work: &'a OpenWorkReply,
        archive_artifact_growth: &'a ArchiveArtifactGrowthOutcome,
    }
    let mut output = std::io::stdout().lock();
    serde_json::to_writer_pretty(
        &mut output,
        &Sample {
            open_work: reply,
            archive_artifact_growth: growth,
        },
    )?;
    writeln!(output)?;
    Ok(())
}
