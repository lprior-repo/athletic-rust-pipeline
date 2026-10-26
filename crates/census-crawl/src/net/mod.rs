use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use tracing::warn;

use census_domain::model::{AccessBlockKind, SourceAccessCondition};
use destination_guard::DestinationGuard;

pub mod bridge;

mod access_conditions;
mod acquisition;
pub mod cache;
mod capture_error;
mod recovery_error;
pub use recovery_error::{RecoveryCleanupFailure, RecoveryCleanupLeaf, RecoveryFailurePrimary};
mod client;
#[cfg(test)]
pub(crate) use acquisition::tests::fixture as capture_fixture;
pub(crate) use acquisition::CaptureProvenance;
pub use acquisition::{
    assess_directory_roster_child, CaptureProvenanceView, CapturedFetch, CapturedPublicDirectory,
    CheckedDirectoryRosterWitnessV1, EligiblePublicationOutcome, EligiblePublicationSkip,
    PreparedRecoveryOutcome, PublicResourceAcquisition, RetainedObservedCaptureView,
};
pub use capture_error::CaptureTransportError;
mod destination_guard;
mod durable_capture;
pub use durable_capture::durable_capture_context;
mod execute;
mod latency;
mod origin_locks;
mod public_http;
mod representation;
mod request;
mod restoration;
mod time;

#[cfg(test)]
#[path = "time_tests.rs"]
mod time_tests;
mod types;

pub use time::{cooldown_until_iso8601, instant_iso8601, now_iso8601, today_iso};
pub use types::{
    EvidenceLane, FetchError, FetchOptions, FetchOutcome, FetchStats, HostTraffic, RedirectHop,
    RefusalEvidenceLoss,
};
mod activity;
pub use activity::{
    sample_archive_artifacts, ArchiveActivityReason, ArchiveActivitySample, ArchiveActivityScan,
    ARCHIVE_ACTIVITY_METRIC_VERSION,
};

pub use representation::RepresentationHeaders;
pub(crate) use types::host_of;

pub use client::PacingState;
pub use origin_locks::origin_lock_file_name;
use origin_locks::OriginLocks;

pub const DEFAULT_USER_AGENT: &str =
    "census-service/0.1 (independent HS track & field research collector; polite; contact: repo owner)";

pub(crate) const MAX_BODY_BYTES: usize = 32 * 1024 * 1024;
const REQUEST_TIMEOUT_SECS: u64 = 45;

const MIN_AUTHORIZED_DELAY: Duration = Duration::from_millis(500);

pub const BLOCK_COOLDOWN_SECONDS: u64 = 6 * 60 * 60;
pub const SHORT_COOLDOWN_SECONDS: u64 = 60;

pub const MAX_EVIDENCE_LOSSES: usize = 4096;

pub const DEFAULT_FAMILY_PARALLELISM: usize = 1;

const DEFAULT_SOURCE: &str = "unknown";

pub struct Fetcher {
    client: reqwest::Client,
    effective_user_agent: String,
    capture_clock: time::CaptureClock,
    #[cfg(test)]
    capture_interrupt: acquisition::CaptureInterruption,
    destination: Arc<DestinationGuard>,
    cache_dir: PathBuf,
    default_delay: Duration,
    host_delays: HashMap<String, Duration>,
    family_delays: HashMap<String, Duration>,
    family_parallelism: usize,
    pacing: Arc<PacingState>,
    authorized_hosts: Vec<String>,
    stats: Mutex<FetchStats>,
    evidence_losses: Mutex<Vec<RefusalEvidenceLoss>>,
    source: String,
    lane: Option<bridge::BrowserLane>,
    offline: bool,
    origin_locks: OriginLocks,
}

impl Fetcher {
    pub fn is_authorized_host(&self, host: &str) -> bool {
        let host = host.trim().to_ascii_lowercase();
        self.authorized_hosts
            .iter()
            .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")))
    }

    pub fn with_family_budgets(mut self, families: HashMap<String, Duration>) -> Self {
        self.family_delays = families
            .into_iter()
            .map(|(family, delay)| (family.trim().to_ascii_lowercase(), delay))
            .filter(|(family, _)| !family.is_empty())
            .collect();
        self
    }

    pub fn with_family_parallelism(mut self, parallelism: usize) -> Self {
        self.family_parallelism = parallelism.max(DEFAULT_FAMILY_PARALLELISM);
        self
    }

    pub fn family_parallelism(&self) -> usize {
        self.family_parallelism
    }

    pub(super) fn family_of(&self, host: &str) -> Option<String> {
        let host = host.trim().to_ascii_lowercase();
        self.family_delays
            .keys()
            .filter(|family| host == family.as_str() || host.ends_with(&format!(".{family}")))
            .max_by_key(|family| family.len())
            .cloned()
    }

    pub(super) fn family_delay(&self, family: &str) -> Option<Duration> {
        self.family_delays.get(family).copied()
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    #[tracing::instrument(skip(self))]
    pub async fn stats(&self) -> FetchStats {
        self.stats.lock().await.clone()
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = source.into();
        self
    }

    pub fn with_browser_lane(mut self, lane: bridge::BrowserLane) -> Self {
        self.lane = Some(lane);
        self
    }

    pub fn with_shared_pacing(mut self, shared: Arc<PacingState>) -> Self {
        self.pacing = shared;
        self
    }

    pub fn pacing_state(&self) -> Arc<PacingState> {
        Arc::clone(&self.pacing)
    }

    pub fn has_browser_lane(&self) -> bool {
        self.lane.is_some()
    }
    pub fn with_offline(mut self, offline: bool) -> Self {
        self.offline = offline;
        self
    }

    pub fn with_origin_locks(mut self, root: impl Into<PathBuf>) -> Self {
        self.origin_locks = OriginLocks::rooted(root.into());
        self
    }

    pub fn claim_origin(&self, origin: &str) -> Result<(), FetchError> {
        self.origin_locks.ensure(origin)
    }

    pub fn is_offline(&self) -> bool {
        self.offline
    }

    pub async fn record_access_condition(
        &self,
        host: &str,
        kind: AccessBlockKind,
        status: u16,
        retry_after_seconds: Option<u64>,
        detail: impl Into<String>,
    ) -> SourceAccessCondition {
        let host = host.trim().to_ascii_lowercase();
        let cooldown_until = match kind {
            AccessBlockKind::HumanRequired => None,
            AccessBlockKind::RateLimited if retry_after_seconds.is_none() => {
                Some(cooldown_until_iso8601(SHORT_COOLDOWN_SECONDS))
            }
            _ => Some(cooldown_until_iso8601(
                retry_after_seconds.map_or(BLOCK_COOLDOWN_SECONDS, |value| value),
            )),
        };
        let condition = SourceAccessCondition::new(
            self.source.clone(),
            host,
            kind,
            status,
            now_iso8601(),
            detail,
        )
        .with_retry_after(retry_after_seconds)
        .with_cooldown_until(cooldown_until);
        if matches!(kind, AccessBlockKind::HumanRequired) {
            let mut stats = self.stats.lock().await;
            stats.challenges = stats.challenges.saturating_add(1);
        }
        let mut blocks = self.pacing.blocks.lock().await;
        blocks.insert(condition.id.clone(), condition.clone());
        condition
    }

    pub async fn record_refusal_evidence_loss(
        &self,
        url: &str,
        status: u16,
        lane: EvidenceLane,
        detail: impl Into<String>,
    ) -> RefusalEvidenceLoss {
        let loss = RefusalEvidenceLoss {
            source: self.source.clone(),
            url: url.to_string(),
            status,
            lane,
            detail: detail.into(),
            observed_at: now_iso8601(),
        };
        let mut losses = self.evidence_losses.lock().await;
        if losses.len() < MAX_EVIDENCE_LOSSES {
            losses.push(loss.clone());
        } else {
            warn!(
                status,
                url, "refusal evidence loss exceeded the {MAX_EVIDENCE_LOSSES}-record report bound"
            );
        }
        loss
    }

    pub async fn refusal_evidence_losses(&self) -> Vec<RefusalEvidenceLoss> {
        let losses = self.evidence_losses.lock().await;
        losses.clone()
    }

    pub async fn blocked_hosts(&self, now_iso8601: &str) -> Vec<String> {
        let blocks = self.pacing.blocks.lock().await;
        let mut hosts: Vec<String> = blocks
            .values()
            .filter(|condition| condition.is_blocking(now_iso8601))
            .map(|condition| condition.host.clone())
            .collect();
        hosts.sort();
        hosts.dedup();
        hosts
    }

    pub async fn host_blocked(&self, host: &str, now_iso8601: &str) -> bool {
        let host = host.trim().to_ascii_lowercase();
        let blocks = self.pacing.blocks.lock().await;
        blocks
            .values()
            .any(|condition| condition.host == host && condition.is_blocking(now_iso8601))
    }
}

#[cfg(test)]
mod tests;
