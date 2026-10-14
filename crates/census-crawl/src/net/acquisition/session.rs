use super::observation::{conditional, failure_of, observation_of};
use super::CaptureProvenance;
use crate::net::cache::{CacheMeta, PinnedEligibleCache};
use crate::net::{CaptureTransportError, FetchError};
use census_domain::model::{AcquisitionFailureV1, DispatchRequestV1};
use census_store::{ArmedDispatch, DispatchObservation, ObservedResponse, ReservedAttempt, Store};

pub(in crate::net) enum CaptureSession<'a> {
    Legacy,
    Adopted(Box<AdoptedSession<'a>>),
}

pub(in crate::net) struct AdoptedSession<'a> {
    pub(super) store: &'a Store,
    pub(super) stage: Stage,
    pub(super) cached: Option<PinnedEligibleCache>,
    #[cfg(test)]
    pub(super) interruption: super::CaptureInterruption,
}

pub(in crate::net) enum Stage {
    Reserved(ReservedAttempt),
    Observed(ObservedResponse),
    Complete(CaptureProvenance),
    Consumed,
}

pub(in crate::net) enum ArmedLease<'a> {
    Legacy,
    Adopted {
        store: &'a Store,
        armed: Box<ArmedDispatch>,
    },
}

impl<'a> ArmedLease<'a> {
    pub(in crate::net) fn into_observation(self) -> DispatchLease<'a> {
        match self {
            Self::Legacy => DispatchLease::Legacy,
            Self::Adopted { store, armed } => DispatchLease::Adopted {
                store,
                observation: Box::new((*armed).into_observation()),
            },
        }
    }
}

pub(in crate::net) enum DispatchLease<'a> {
    Legacy,
    Adopted {
        store: &'a Store,
        observation: Box<DispatchObservation>,
    },
}

impl<'a> CaptureSession<'a> {
    pub(in crate::net) fn new(
        store: &'a Store,
        attempt: ReservedAttempt,
        cached: Option<PinnedEligibleCache>,
    ) -> Self {
        Self::Adopted(Box::new(AdoptedSession {
            store,
            stage: Stage::Reserved(attempt),
            cached,
            #[cfg(test)]
            interruption: super::CaptureInterruption::None,
        }))
    }

    pub(in crate::net) fn is_adopted(&self) -> bool {
        matches!(self, Self::Adopted(_))
    }

    pub(in crate::net) fn cached(&self) -> Option<&CacheMeta> {
        match self {
            Self::Legacy => None,
            Self::Adopted(adopted) => adopted
                .cached
                .as_ref()
                .map(|value| &value.envelope.metadata),
        }
    }

    pub(in crate::net) fn arm(
        &mut self,
        request: &reqwest::Request,
    ) -> Result<ArmedLease<'a>, FetchError> {
        let Self::Adopted(adopted) = self else {
            return Ok(ArmedLease::Legacy);
        };
        let AdoptedSession {
            store,
            stage,
            cached,
            ..
        } = adopted.as_mut();
        let conditional = conditional(request, cached.as_ref())?;
        let dispatch = DispatchRequestV1::new(request.url().as_str(), conditional)?;
        let armed = match std::mem::replace(stage, Stage::Consumed) {
            Stage::Reserved(attempt) => store.arm_acquisition_dispatch(attempt, dispatch)?,
            Stage::Observed(response) => store.arm_acquisition_redirect(response, dispatch)?,
            Stage::Complete(_) | Stage::Consumed => {
                return Err(CaptureTransportError::DispatchConsumed.into())
            }
        };
        Ok(ArmedLease::Adopted {
            store,
            armed: Box::new(armed),
        })
    }

    pub(in crate::net) fn observed(
        &mut self,
        lease: DispatchLease<'_>,
        response: &reqwest::Response,
        observed_at: &str,
    ) -> Result<(), FetchError> {
        let DispatchLease::Adopted { store, observation } = lease else {
            return Ok(());
        };
        let observed = match observation_of(response, observed_at) {
            Ok(value) => value,
            Err(error) => {
                store.refuse_acquisition_response_checkpoint(
                    *observation,
                    response.status().as_u16(),
                    response.url().as_str(),
                    census_domain::model::ResponseCheckpointRefusalV1::UnrepresentableResponse,
                )?;
                return Err(error);
            }
        };
        let retained = store.record_acquisition_response(*observation, observed)?;
        match self {
            Self::Adopted(adopted) => {
                adopted.stage = Stage::Observed(retained);
                Ok(())
            }
            Self::Legacy => Err(CaptureTransportError::DispatchConsumed.into()),
        }
    }

    pub(in crate::net) fn observed_at(&self) -> Result<Option<&str>, FetchError> {
        match self {
            Self::Legacy => Ok(None),
            Self::Adopted(adopted) => match &adopted.stage {
                Stage::Observed(response) => Ok(Some(response.response().observed_at())),
                _ => Err(CaptureTransportError::DispatchConsumed.into()),
            },
        }
    }

    pub(in crate::net) fn provenance(self) -> Result<CaptureProvenance, FetchError> {
        match self {
            Self::Legacy => Ok(CaptureProvenance::HistoricalAttemptUnknown),
            Self::Adopted(adopted) => match adopted.stage {
                Stage::Complete(provenance) => Ok(provenance),
                _ => Err(CaptureTransportError::DispatchConsumed.into()),
            },
        }
    }

    pub(in crate::net) fn fail(&mut self, error: &FetchError) -> Result<(), FetchError> {
        if matches!(self, Self::Adopted(adopted) if matches!(&adopted.stage, Stage::Reserved(_)))
            && matches!(error, FetchError::Timeout { .. })
        {
            return Ok(());
        }
        self.refuse(failure_of(error))
    }

    pub(in crate::net) fn refuse(
        &mut self,
        failure: AcquisitionFailureV1,
    ) -> Result<(), FetchError> {
        let Self::Adopted(adopted) = self else {
            return Ok(());
        };
        let AdoptedSession { store, stage, .. } = adopted.as_mut();
        match std::mem::replace(stage, Stage::Consumed) {
            Stage::Reserved(attempt) => {
                store.refuse_acquisition_attempt(attempt, failure)?;
            }
            Stage::Observed(response) => {
                store.fail_acquisition_response(response, failure)?;
            }
            Stage::Complete(provenance) => {
                *stage = Stage::Complete(provenance);
            }
            Stage::Consumed => {}
        }
        Ok(())
    }
}

impl DispatchLease<'_> {
    pub(in crate::net) fn fail(self, error: &FetchError) -> Result<(), FetchError> {
        if let Self::Adopted { store, observation } = self {
            store.fail_acquisition_dispatch(*observation, failure_of(error))?;
        }
        Ok(())
    }
}
