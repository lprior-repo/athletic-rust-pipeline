use super::{error, presentation, Execution, ResultsPlanV2};
use crate::restate_services::jobs;
use census_crawl::{
    net::{CapturedFetch, EligiblePublicationOutcome, PreparedRecoveryOutcome},
    ResultsCaptureContext,
};
use census_domain::model::{AcquisitionWorkV1, PreparedRecoveryTarget};
use census_store::{ExactBodySource, PreparedRecoveryBudget, Receipt};
use restate_sdk::prelude::HandlerError;

pub(super) type Resolution = Result<RetainedResolution, ResolutionFailure>;

pub(super) struct RetainedResolution {
    pub(super) context: ResultsCaptureContext,
    pub(super) notice: String,
}

pub(super) enum ResolutionFailure {
    Unavailable { notice: String },
    Failed { notice: String },
    Refused { detail: String },
}

impl Execution<'_> {
    pub(super) fn resolve(
        &self,
        plan: &ResultsPlanV2,
        index: usize,
        pin: PreparedRecoveryTarget,
        budget: &mut PreparedRecoveryBudget,
    ) -> Result<Resolution, HandlerError> {
        let work = self.group_work(plan, index)?;
        let result = self.fetcher.recover_prepared_target(
            self.store,
            self.owner,
            &pin,
            budget,
            ExactBodySource::NoneSupplied,
        );
        match result {
            Ok(PreparedRecoveryOutcome::Retained {
                captured,
                terminal: _,
                receipt,
                eligible,
            }) => self.retained(pin, work, captured, receipt, eligible),
            Ok(PreparedRecoveryOutcome::RetainedUnavailable {
                occurrence: _,
                terminal: _,
                receipt,
                consumption_error,
            }) => {
                let identity = presentation::identity(&pin, &receipt, "retained_unavailable")?;
                Ok(Err(ResolutionFailure::Unavailable {
                    notice: presentation::text(&format_args!(
                        "{identity}; consumption error: {consumption_error}"
                    ))?,
                }))
            }
            Ok(PreparedRecoveryOutcome::RetentionFailed {
                terminal: _,
                receipt,
            }) => {
                let identity = presentation::identity(&pin, &receipt, "retention_failed")?;
                Ok(Err(ResolutionFailure::Failed {
                    notice: presentation::text(&format_args!(
                        "{identity}; original-owner continuation requires a later invocation"
                    ))?,
                }))
            }
            Err(error) => Ok(Err(ResolutionFailure::Refused {
                detail: presentation::text(&error)?,
            })),
        }
    }

    fn retained(
        &self,
        pin: PreparedRecoveryTarget,
        work: AcquisitionWorkV1,
        captured: Box<CapturedFetch>,
        receipt: Receipt,
        eligible: EligiblePublicationOutcome,
    ) -> Result<Resolution, HandlerError> {
        let identity = presentation::identity(&pin, &receipt, "retained")?;
        let publication = presentation::publication(eligible)?;
        let notice = presentation::text(&format_args!("{identity}; {publication}"))?;
        match ResultsCaptureContext::from_prepared_recovery(self.owner.clone(), pin, work, captured)
        {
            Ok(context) => Ok(Ok(RetainedResolution { context, notice })),
            Err(error) => Ok(Err(ResolutionFailure::Unavailable {
                notice: presentation::text(&format_args!(
                    "{notice}; retained consumption unavailable: {error}"
                ))?,
            })),
        }
    }

    fn group_work(
        &self,
        plan: &ResultsPlanV2,
        pin: usize,
    ) -> Result<AcquisitionWorkV1, HandlerError> {
        let mut selected = None;
        for (index, route) in plan.routes.iter().enumerate() {
            if plan.pin_for(route).map_err(error)? == Some(pin) {
                selected = Some(index);
                break;
            }
        }
        let meet = selected
            .and_then(|index| plan.meets.get(index))
            .ok_or_else(|| jobs::invariant("fixed pin has no selected meet"))?;
        let reference = super::super::super::capture_scope::checked_target(
            meet,
            &census_crawl::milesplit::ResultSetOptions::default(),
        )
        .map_err(|_| jobs::invariant("fixed pin target derivation"))?;
        let options = super::super::request::options(self.store, self.fetcher, self.scope, meet)?;
        let request =
            census_crawl::milesplit::owned_meet_request(self.fetcher, &reference, &options)
                .map_err(|error| crate::restate_services::job_error(jobs::collect_error(error)))?;
        self.owner
            .for_owned_meet(reference.site.jurisdiction(), &reference.meet_id, request)
            .map_err(|error| crate::restate_services::job_error(error.into()))
    }
}
