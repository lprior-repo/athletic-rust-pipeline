use census_domain::UsJurisdiction;
use serde::{Deserialize, Serialize};

use crate::census::JurisdictionStages;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenWorkRequest {
    pub season: i16,
    #[serde(default = "first_revision")]
    pub revision: u32,
    #[serde(default)]
    pub source_objects: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_page: Option<SourceObjectPageRequest>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceObjectPageRequest {
    #[serde(default)]
    pub after: Option<String>,
    #[serde(default)]
    pub limit: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceObjectPageReply {
    pub total: u64,
    pub offset: u64,
    pub returned: u64,
    pub next: Option<String>,
}

fn first_revision() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JurisdictionOpen {
    pub jurisdiction: UsJurisdiction,
    pub identity: String,
    pub stages: JurisdictionStages,
    pub unreadable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teams_stage: Option<super::TeamsStage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_refusals: Option<Vec<super::RefusedSource>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub athleticlive_manifest: Option<super::AthleticLiveObligation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub athleticlive_manifest_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceObjectOpen {
    pub endpoint: String,
    pub observations: u64,
    pub windows: u64,
    pub unreadable: bool,
    #[serde(default)]
    pub disposition: census_crawl::CollectionDisposition,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<census_crawl::ResolutionCounters>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teams_source: Option<super::TeamsSourceOutcome>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceObjectInventory {
    pub objects: u64,
    pub accepted_observations: u64,
    pub accepted_nothing: u64,
    pub unreadable: u64,
}

pub use census_crawl::net::ARCHIVE_ACTIVITY_METRIC_VERSION;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ArchiveUnmeasuredReason {
    FenceBusy,
    Missing,
    LockIo,
    Io,
    UnexpectedLayout,
    Corrupt,
    Limit,
    Unsupported,
    StoreIdentityUnavailable,
    Legacy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveCaptureEpoch {
    pub store_identity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_dev: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_ino: Option<u64>,
    pub metric_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveCaptureSample {
    pub epoch: ArchiveCaptureEpoch,
    pub started_at: String,
    pub finished_at: String,
    pub capture_documents: u64,
    pub unique_body_files: u64,
    pub unique_body_payload_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveCaptureUnmeasured {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epoch: Option<ArchiveCaptureEpoch>,
    pub started_at: String,
    pub finished_at: String,
    pub reason: ArchiveUnmeasuredReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum ArchiveCaptureActivity {
    Measured(ArchiveCaptureSample),
    Unmeasured(ArchiveCaptureUnmeasured),
}

impl Default for ArchiveCaptureActivity {
    fn default() -> Self {
        Self::Unmeasured(ArchiveCaptureUnmeasured {
            epoch: None,
            started_at: String::new(),
            finished_at: String::new(),
            reason: ArchiveUnmeasuredReason::Legacy,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenWorkReply {
    pub season: String,
    pub revision: u32,
    pub jurisdiction_sweeps: Option<u64>,
    pub source_objects: Option<u64>,
    #[serde(default)]
    pub source_object_inventory: Option<SourceObjectInventory>,
    #[serde(default)]
    pub silent_sources: Vec<String>,
    pub jurisdictions: Vec<JurisdictionOpen>,
    pub endpoints: Vec<SourceObjectOpen>,
    #[serde(default)]
    pub archive_capture_activity: ArchiveCaptureActivity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_page: Option<SourceObjectPageReply>,
}
