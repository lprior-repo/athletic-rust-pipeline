use super::{CaptureCacheIntent, SourceAdmissionV1};
use crate::model::source_acquisition::scalar::{bounded, try_text};
use crate::model::{AcquisitionDigest, CaptureDomainError, CensusRun, HistoryWindow};
use crate::UsJurisdiction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublicResourceCaptureScopeV1 {
    owner_key: String,
    jurisdiction: UsJurisdiction,
    run: CensusRun,
    run_manifest_digest: AcquisitionDigest,
    history: HistoryWindow,
    source_admission: SourceAdmissionV1,
    cache_intent: CaptureCacheIntent,
}

#[cfg(test)]
mod tests;

impl PublicResourceCaptureScopeV1 {
    pub fn new(
        owner_key: &str,
        jurisdiction: UsJurisdiction,
        run: CensusRun,
        run_manifest_digest: AcquisitionDigest,
        history: HistoryWindow,
        frontier_plan_digest: AcquisitionDigest,
        cache_intent: CaptureCacheIntent,
    ) -> Result<Self, CaptureDomainError> {
        bounded(owner_key, "owner key", 64)?;
        Ok(Self {
            owner_key: owner_key.to_owned(),
            jurisdiction,
            run,
            run_manifest_digest,
            history,
            source_admission: SourceAdmissionV1::public_resources(frontier_plan_digest.as_str())?,
            cache_intent,
        })
    }
    pub fn try_clone(&self) -> Result<Self, std::collections::TryReserveError> {
        Ok(Self {
            owner_key: try_text(&self.owner_key)?,
            jurisdiction: self.jurisdiction,
            run: self.run,
            run_manifest_digest: self.run_manifest_digest.try_clone()?,
            history: self.history,
            source_admission: self.source_admission.try_clone()?,
            cache_intent: self.cache_intent,
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
    pub fn source_admission(&self) -> &SourceAdmissionV1 {
        &self.source_admission
    }
    pub fn cache_intent(&self) -> CaptureCacheIntent {
        self.cache_intent
    }
}
