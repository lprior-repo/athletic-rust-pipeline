use super::qualification::CaptureProtocol;
use super::scalar::{bounded, VersionOne};
use super::{
    AcquisitionDigest, AcquisitionSlotV1, AcquisitionWorkId, CaptureDomainError,
    RequestRepresentationV1,
};
use crate::model::{serialized_digest, CensusRun, HistoryWindow, StoreIdentity};
use crate::UsJurisdiction;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureCacheIntent {
    ReuseEligible,
    AcquireNew,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AdmissionKind {
    LegacyRestateSourceSelectionV1,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum StateKey {
    State,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SelectedSource {
    Milesplit,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SelectedArm {
    MilesplitResults,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DurableRole {
    StructuredWholeMeet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceAdmissionV1 {
    kind: AdmissionKind,
    state_key: StateKey,
    plan_fingerprint: AcquisitionDigest,
    selected_source: SelectedSource,
    selected_arm: SelectedArm,
    durable_role: DurableRole,
}
impl SourceAdmissionV1 {
    pub fn try_clone(&self) -> Result<Self, std::collections::TryReserveError> {
        Ok(Self {
            kind: self.kind,
            state_key: self.state_key,
            plan_fingerprint: self.plan_fingerprint.try_clone()?,
            selected_source: self.selected_source,
            selected_arm: self.selected_arm,
            durable_role: self.durable_role,
        })
    }
    pub fn milesplit_results(plan_fingerprint: &str) -> Result<Self, CaptureDomainError> {
        Ok(Self {
            kind: AdmissionKind::LegacyRestateSourceSelectionV1,
            state_key: StateKey::State,
            plan_fingerprint: AcquisitionDigest::parse(plan_fingerprint)?,
            selected_source: SelectedSource::Milesplit,
            selected_arm: SelectedArm::MilesplitResults,
            durable_role: DurableRole::StructuredWholeMeet,
        })
    }
    pub fn plan_fingerprint(&self) -> &AcquisitionDigest {
        &self.plan_fingerprint
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectedHistoryV1 {
    history: HistoryWindow,
    historical_year: u16,
}
impl SelectedHistoryV1 {
    pub fn new(history: HistoryWindow, historical_year: u16) -> Result<Self, CaptureDomainError> {
        if !history.years().contains(&historical_year) {
            return Err(CaptureDomainError::HistoryConflict);
        }
        Ok(Self {
            history,
            historical_year,
        })
    }
    pub fn history(self) -> HistoryWindow {
        self.history
    }
    pub fn historical_year(self) -> u16 {
        self.historical_year
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultsCaptureScopeV1 {
    owner_key: String,
    jurisdiction: UsJurisdiction,
    run: CensusRun,
    run_manifest_digest: AcquisitionDigest,
    history: HistoryWindow,
    historical_year: u16,
    source_admission: SourceAdmissionV1,
    cache_intent: CaptureCacheIntent,
}
impl ResultsCaptureScopeV1 {
    pub fn try_clone(&self) -> Result<Self, std::collections::TryReserveError> {
        Ok(Self {
            owner_key: super::scalar::try_text(&self.owner_key)?,
            jurisdiction: self.jurisdiction,
            run: self.run,
            run_manifest_digest: self.run_manifest_digest.try_clone()?,
            history: self.history,
            historical_year: self.historical_year,
            source_admission: self.source_admission.try_clone()?,
            cache_intent: self.cache_intent,
        })
    }
    pub fn new(
        owner_key: &str,
        jurisdiction: UsJurisdiction,
        run: CensusRun,
        run_manifest_digest: AcquisitionDigest,
        selected_history: SelectedHistoryV1,
        source_admission: SourceAdmissionV1,
        cache_intent: CaptureCacheIntent,
    ) -> Result<Self, CaptureDomainError> {
        bounded(owner_key, "owner key", 64)?;
        Ok(Self {
            owner_key: owner_key.to_string(),
            jurisdiction,
            run,
            run_manifest_digest,
            history: selected_history.history(),
            historical_year: selected_history.historical_year(),
            source_admission,
            cache_intent,
        })
    }
    pub fn owner_key(&self) -> &str {
        &self.owner_key
    }
    pub fn jurisdiction(&self) -> UsJurisdiction {
        self.jurisdiction
    }
    pub fn run(&self) -> CensusRun {
        self.run
    }
    pub fn run_manifest_digest(&self) -> &AcquisitionDigest {
        &self.run_manifest_digest
    }
    pub fn history(&self) -> HistoryWindow {
        self.history
    }
    pub fn historical_year(&self) -> u16 {
        self.historical_year
    }
    pub fn source_admission(&self) -> &SourceAdmissionV1 {
        &self.source_admission
    }
    pub fn cache_intent(&self) -> CaptureCacheIntent {
        self.cache_intent
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "WorkWire")]
pub struct AcquisitionWorkV1 {
    version: VersionOne,
    slot: AcquisitionSlotV1,
    run: CensusRun,
    run_manifest_digest: AcquisitionDigest,
    history: HistoryWindow,
    source_admission: SourceAdmissionV1,
    request: RequestRepresentationV1,
    cache_intent: CaptureCacheIntent,
    protocol: CaptureProtocol,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WorkWire {
    version: VersionOne,
    slot: AcquisitionSlotV1,
    run: CensusRun,
    run_manifest_digest: AcquisitionDigest,
    history: HistoryWindow,
    source_admission: SourceAdmissionV1,
    request: RequestRepresentationV1,
    cache_intent: CaptureCacheIntent,
    protocol: CaptureProtocol,
}
impl AcquisitionWorkV1 {
    pub fn for_owned_meet(
        identity: StoreIdentity,
        scope: &ResultsCaptureScopeV1,
        site: UsJurisdiction,
        meet_id: &str,
        request: RequestRepresentationV1,
    ) -> Result<Self, CaptureDomainError> {
        let slot = AcquisitionSlotV1::new(
            identity,
            scope.owner_key(),
            scope.historical_year(),
            site,
            meet_id,
        )?;
        let result = Self {
            version: VersionOne,
            slot,
            run: scope.run,
            run_manifest_digest: scope.run_manifest_digest.clone(),
            history: scope.history,
            source_admission: scope.source_admission.clone(),
            request,
            cache_intent: scope.cache_intent,
            protocol: CaptureProtocol::SourceAcquisitionAttemptV1,
        };
        result.validate()?;
        Ok(result)
    }
    fn validate(&self) -> Result<(), CaptureDomainError> {
        if !self.history.years().contains(&self.slot.historical_year()) {
            return Err(CaptureDomainError::HistoryConflict);
        }
        let url = super::scalar::url(self.request.original_url())?;
        let host = format!(
            "{}.milesplit.com",
            self.slot.source_site().code().to_ascii_lowercase()
        );
        let path = format!("/api/v1/meets/{}/performances", self.slot.source_meet_id());
        if url.scheme() != "https"
            || url.host_str() != Some(host.as_str())
            || url.port().is_some()
            || url.path() != path
            || url.query().is_none()
        {
            return Err(CaptureDomainError::RequestTargetMismatch);
        }
        Ok(())
    }
    pub fn id(&self) -> Result<AcquisitionWorkId, CaptureDomainError> {
        let digest = serialized_digest(&("source-acquisition-work-v1", self))
            .map_err(|e| CaptureDomainError::Encoding(e.to_string()))?;
        AcquisitionWorkId::parse(&digest)
    }
    pub fn slot(&self) -> &AcquisitionSlotV1 {
        &self.slot
    }
    pub fn run(&self) -> CensusRun {
        self.run
    }
    pub fn run_manifest_digest(&self) -> &AcquisitionDigest {
        &self.run_manifest_digest
    }
    pub fn history(&self) -> HistoryWindow {
        self.history
    }
    pub fn source_admission(&self) -> &SourceAdmissionV1 {
        &self.source_admission
    }
    pub fn request(&self) -> &RequestRepresentationV1 {
        &self.request
    }
    pub fn cache_intent(&self) -> CaptureCacheIntent {
        self.cache_intent
    }
}
impl TryFrom<WorkWire> for AcquisitionWorkV1 {
    type Error = CaptureDomainError;
    fn try_from(w: WorkWire) -> Result<Self, Self::Error> {
        let result = Self {
            version: w.version,
            slot: w.slot,
            run: w.run,
            run_manifest_digest: w.run_manifest_digest,
            history: w.history,
            source_admission: w.source_admission,
            request: w.request,
            cache_intent: w.cache_intent,
            protocol: w.protocol,
        };
        result.validate()?;
        Ok(result)
    }
}
