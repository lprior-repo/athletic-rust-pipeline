use super::{bound_run_refusal, capture_admission, jobs, JurisdictionCensus};
use crate::restate_services::wire::JurisdictionRequest;
use restate_sdk::prelude::*;

impl JurisdictionCensus {
    pub(super) fn require_bound_run(
        &self,
        request: &JurisdictionRequest,
    ) -> Result<(), HandlerError> {
        let manifest = self.store.run_manifest().map_err(|error| {
            jobs::invariant(&format!("the store run manifest is unreadable: {error}"))
        })?;
        capture_admission::require_bound_manifest(&self.capture, manifest.as_ref(), request)
            .map_err(capture_admission::handler_error)?;
        match bound_run_refusal(manifest.as_ref(), request.season, request.revision) {
            Some(refusal) => Err(TerminalError::new(refusal).into()),
            None => Ok(()),
        }
    }
}
