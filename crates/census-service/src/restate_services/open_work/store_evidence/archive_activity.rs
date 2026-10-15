use crate::restate_services::wire::{
    ArchiveCaptureActivity, ArchiveCaptureEpoch, ArchiveCaptureSample, ArchiveCaptureUnmeasured,
    ArchiveUnmeasuredReason, ARCHIVE_ACTIVITY_METRIC_VERSION,
};
use census_store::clock::Clock;
use census_store::Store;

pub(in crate::restate_services) fn sample_archive_activity(
    store: &Store,
    started_at: &str,
    clock: &dyn Clock,
) -> ArchiveCaptureActivity {
    let identity = match store.store_identity() {
        Ok(identity) => identity.as_str().to_string(),
        Err(_) => {
            return ArchiveCaptureActivity::Unmeasured(ArchiveCaptureUnmeasured {
                epoch: None,
                started_at: started_at.to_string(),
                finished_at: clock.today_iso8601(),
                reason: ArchiveUnmeasuredReason::StoreIdentityUnavailable,
            })
        }
    };
    let http_root = store.http_cache_dir();
    let scan = census_crawl::net::sample_archive_artifacts(&http_root, started_at, clock);
    match scan {
        census_crawl::net::ArchiveActivityScan::Measured(sample) => {
            ArchiveCaptureActivity::Measured(ArchiveCaptureSample {
                epoch: ArchiveCaptureEpoch {
                    store_identity: identity,
                    archive_dev: Some(sample.archive_dev),
                    archive_ino: Some(sample.archive_ino),
                    metric_version: ARCHIVE_ACTIVITY_METRIC_VERSION,
                },
                started_at: sample.started_at,
                finished_at: sample.finished_at,
                capture_documents: sample.capture_documents,
                unique_body_files: sample.unique_body_files,
                unique_body_payload_bytes: sample.unique_body_payload_bytes,
            })
        }
        census_crawl::net::ArchiveActivityScan::Unmeasured { reason } => {
            ArchiveCaptureActivity::Unmeasured(ArchiveCaptureUnmeasured {
                epoch: Some(ArchiveCaptureEpoch {
                    store_identity: identity,
                    archive_dev: None,
                    archive_ino: None,
                    metric_version: ARCHIVE_ACTIVITY_METRIC_VERSION,
                }),
                started_at: started_at.to_string(),
                finished_at: clock.today_iso8601(),
                reason: map_reason(reason),
            })
        }
    }
}

fn map_reason(reason: census_crawl::net::ArchiveActivityReason) -> ArchiveUnmeasuredReason {
    match reason {
        census_crawl::net::ArchiveActivityReason::FenceBusy => ArchiveUnmeasuredReason::FenceBusy,
        census_crawl::net::ArchiveActivityReason::Missing => ArchiveUnmeasuredReason::Missing,
        census_crawl::net::ArchiveActivityReason::LockIo => ArchiveUnmeasuredReason::LockIo,
        census_crawl::net::ArchiveActivityReason::Io => ArchiveUnmeasuredReason::Io,
        census_crawl::net::ArchiveActivityReason::UnexpectedLayout => {
            ArchiveUnmeasuredReason::UnexpectedLayout
        }
        census_crawl::net::ArchiveActivityReason::Corrupt => ArchiveUnmeasuredReason::Corrupt,
        census_crawl::net::ArchiveActivityReason::Limit => ArchiveUnmeasuredReason::Limit,
        census_crawl::net::ArchiveActivityReason::Unsupported => {
            ArchiveUnmeasuredReason::Unsupported
        }
    }
}
