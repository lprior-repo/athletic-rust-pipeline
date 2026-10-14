use super::ResultsCaptureExecution;
use crate::restate_services::{job_error, jobs};
use census_crawl::milesplit::{ResultSetOptions, ResultSetRef};
use census_crawl::{CaptureContext, ResultsCaptureContext};
use census_domain::model::SourceMeetRef;
use census_store::Store;
use restate_sdk::prelude::HandlerError;

pub(super) enum AdmittedCapture {
    LegacyMetadata,
    DurableV1(ResultsCaptureContext),
}

impl AdmittedCapture {
    pub(super) fn context(&self) -> CaptureContext<'_> {
        match self {
            Self::LegacyMetadata => CaptureContext::LegacyMetadata,
            Self::DurableV1(context) => CaptureContext::DurableV1(context),
        }
    }
}

pub(super) fn admit(
    store: &Store,
    execution: &ResultsCaptureExecution,
    meet: &SourceMeetRef,
    options: &ResultSetOptions,
) -> Result<AdmittedCapture, HandlerError> {
    let (writer, owner, grants) = match execution {
        ResultsCaptureExecution::LegacyFresh => return Ok(AdmittedCapture::LegacyMetadata),
        ResultsCaptureExecution::UnadoptedSources => {
            return Err(jobs::invariant(
                "capture role was excluded by the retained source plan",
            ));
        }
        ResultsCaptureExecution::FreshQualificationV1 {
            writer,
            owner,
            grants,
        } => (writer, owner, grants),
    };
    let target =
        checked_target(meet, options).map_err(|error| jobs::invariant(&error.to_string()))?;
    let slot = owner
        .slot_for_owned_meet(target.site.jurisdiction(), &target.meet_id)
        .map_err(|error| job_error(error.into()))?;
    let identity = slot
        .id()
        .map_err(|error| jobs::invariant(&error.to_string()))?;
    grants
        .claim(identity)
        .map_err(|error| jobs::invariant(&error.to_string()))?;
    let grant = writer
        .continuation_for_owned_meet(store, owner, target.site.jurisdiction(), &target.meet_id)
        .map_err(|error| job_error(error.into()))?;
    let context = ResultsCaptureContext::new(
        owner.clone(),
        grant,
        target.site.jurisdiction(),
        &target.meet_id,
    )
    .map_err(|error| job_error(jobs::collect_error(error)))?;
    Ok(AdmittedCapture::DurableV1(context))
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum TargetAdmissionError {
    #[error("capture result-set frontier exceeds 4096")]
    FrontierLimit,
    #[error("capture result-set target differs from the selected whole meet")]
    OwnerMismatch,
    #[error("capture result-set URL exceeds 4096 bytes")]
    UrlLimit,
    #[error("capture result-set target has no checked MileSplit owner")]
    Unparseable,
}

fn checked_target(
    meet: &SourceMeetRef,
    options: &ResultSetOptions,
) -> Result<ResultSetRef, TargetAdmissionError> {
    let target = reference(&meet.results_url, meet.jurisdiction)?;
    if options.urls.len() > 4096 {
        return Err(TargetAdmissionError::FrontierLimit);
    }
    options.urls.iter().try_for_each(|request| {
        let discovered = reference(&request.url, request.jurisdiction)?;
        if discovered.site != target.site || discovered.meet_id != target.meet_id {
            return Err(TargetAdmissionError::OwnerMismatch);
        }
        Ok(())
    })?;
    Ok(target)
}

fn reference(
    url: &str,
    jurisdiction: census_domain::UsJurisdiction,
) -> Result<ResultSetRef, TargetAdmissionError> {
    if url.len() > 4096 {
        return Err(TargetAdmissionError::UrlLimit);
    }
    ResultSetRef::parse(url)
        .or_else(|| ResultSetRef::parse_with_jurisdiction(url, jurisdiction))
        .ok_or(TargetAdmissionError::Unparseable)
}

#[cfg(test)]
mod tests;
