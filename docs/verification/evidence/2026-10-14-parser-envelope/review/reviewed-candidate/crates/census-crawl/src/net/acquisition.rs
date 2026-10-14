use super::{CaptureTransportError, FetchError, FetchOptions, FetchOutcome, Fetcher};
use census_domain::model::{
    AcquisitionOccurrenceRefV1, CaptureCacheIntent, InterruptionCauseV1, RequestRepresentationV1,
};
use census_store::{AcquireDecision, AcquisitionReadBudget, OwnedAcquisitionWork, Store};

mod observation;
mod read;
mod retention;
mod session;
#[cfg(test)]
mod testing;
#[cfg(test)]
pub(super) use testing::CaptureInterruption;
#[cfg(test)]
pub(crate) mod tests;
pub(super) use session::CaptureSession;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureProvenanceView<'a> {
    HistoricalAttemptUnknown,
    Retained {
        occurrence: &'a AcquisitionOccurrenceRefV1,
        original: &'a AcquisitionOccurrenceRefV1,
    },
}

#[derive(Debug)]
pub(crate) struct CaptureProvenance {
    state: CaptureProvenanceState,
}

#[derive(Debug)]
enum CaptureProvenanceState {
    HistoricalAttemptUnknown,
    Retained {
        occurrence: AcquisitionOccurrenceRefV1,
        original: AcquisitionOccurrenceRefV1,
    },
}

impl CaptureProvenance {
    fn historical() -> Self {
        Self {
            state: CaptureProvenanceState::HistoricalAttemptUnknown,
        }
    }

    fn retained(
        occurrence: AcquisitionOccurrenceRefV1,
        original: AcquisitionOccurrenceRefV1,
    ) -> Self {
        Self {
            state: CaptureProvenanceState::Retained { occurrence, original },
        }
    }

    pub(crate) fn view(&self) -> CaptureProvenanceView<'_> {
        match &self.state {
            CaptureProvenanceState::HistoricalAttemptUnknown => {
                CaptureProvenanceView::HistoricalAttemptUnknown
            }
            CaptureProvenanceState::Retained { occurrence, original } => {
                CaptureProvenanceView::Retained { occurrence, original }
            }
        }
    }
}

#[derive(Debug)]
pub struct CapturedFetch {
    capture: FetchOutcome,
    provenance: CaptureProvenance,
}

impl CapturedFetch {
    pub(crate) fn historical(capture: FetchOutcome) -> Self {
        Self {
            capture,
            provenance: CaptureProvenance::historical(),
        }
    }

    pub fn capture(&self) -> &FetchOutcome {
        &self.capture
    }

    pub fn provenance(&self) -> CaptureProvenanceView<'_> {
        self.provenance.view()
    }

    pub fn into_capture(self) -> FetchOutcome {
        self.capture
    }

    pub(crate) fn into_parser_parts(self) -> (FetchOutcome, CaptureProvenance) {
        (self.capture, self.provenance)
    }
}

impl Fetcher {
    pub(crate) fn acquisition_request(
        &self,
        url: &str,
        options: &FetchOptions,
    ) -> Result<RequestRepresentationV1, FetchError> {
        self.destination.validate_url(url)?;
        let representation =
            super::execute::representation::canonical_request(url, &options.headers)?;
        let agent = representation
            .entries()
            .iter()
            .find(|(name, _)| name == "user-agent")
            .map_or(self.effective_user_agent.as_str(), |(_, value)| {
                value.as_str()
            });
        Ok(RequestRepresentationV1::get(
            url,
            representation.entries().to_vec(),
            agent,
        )?)
    }

    #[tracing::instrument(skip(self, store, work, options))]
    pub(crate) async fn get_acquisition(
        &self,
        store: &Store,
        work: OwnedAcquisitionWork,
        options: &FetchOptions,
    ) -> Result<CapturedFetch, FetchError> {
        let url = work.work().request().original_url().to_owned();
        let intent = if options.refresh {
            CaptureCacheIntent::AcquireNew
        } else {
            CaptureCacheIntent::ReuseEligible
        };
        if self.acquisition_request(&url, options)? != *work.work().request()
            || intent != work.work().cache_intent()
        {
            return Err(CaptureTransportError::RequestMismatch.into());
        }
        let mut budget = AcquisitionReadBudget::for_work();
        match store.inspect_acquisition_work(&work, &mut budget)? {
            AcquireDecision::Replay(reference) => {
                accepted(read::replay(store, &reference)?, options)
            }
            AcquireDecision::FinishPrepared(_) => {
                accepted(self.recover_prepared(store, work)?, options)
            }
            AcquireDecision::RecoveryRequired(_) => {
                store.recover_acquisition_attempt(work, InterruptionCauseV1::OwnerRestarted)?;
                Err(census_store::CaptureError::RetainedOutcomeUnknown.into())
            }
            AcquireDecision::Ready
            | AcquireDecision::ResumeReserved(_)
            | AcquireDecision::RetryOwed(_) => {
                self.acquire_registered(store, work, &url, options).await
            }
            AcquireDecision::Settled(_) => Err(CaptureTransportError::Settled.into()),
            AcquireDecision::Exhausted => {
                Err(census_store::CaptureError::AcquisitionBudgetExhausted.into())
            }
            AcquireDecision::Interrupted => Err(CaptureTransportError::Interrupted.into()),
        }
    }

    #[tracing::instrument(skip(self, store, work, options))]
    async fn acquire_registered(
        &self,
        store: &Store,
        work: OwnedAcquisitionWork,
        url: &str,
        options: &FetchOptions,
    ) -> Result<CapturedFetch, FetchError> {
        let representation =
            super::execute::representation::canonical_request(url, &options.headers)?;
        let extra = super::execute::representation::cache_extra(None, &representation);
        let (body_path, meta_path) = self.cache_paths(&Self::key_for("GET", url, &extra));
        let cached = super::cache::read_eligible(&body_path, &meta_path, url, &representation)?;
        if !options.refresh {
            if let Some(pinned) = cached.as_ref() {
                let mut budget = AcquisitionReadBudget::for_work();
                return pinned.compare(&body_path, &meta_path, || {
                    let mut original_budget = AcquisitionReadBudget::for_work();
                    pinned.verify(store, &mut original_budget, &meta_path)?;
                    let retained = store.reuse_acquisition_cache(
                        work,
                        &pinned.envelope.original_occurrence,
                        &mut budget,
                    )?;
                    pinned.require_retained(&retained, &meta_path)?;
                    read::outcome(retained, true)
                });
            }
        }
        if let Some(pinned) = cached.as_ref() {
            let mut budget = AcquisitionReadBudget::for_work();
            pinned.compare(&body_path, &meta_path, || {
                pinned.verify(store, &mut budget, &meta_path)
            })?;
        }
        let attempt = store.reserve_acquisition_attempt(work)?;
        let mut session = CaptureSession::new(store, attempt, cached);
        #[cfg(test)]
        session.set_interruption(self.capture_interrupt);
        let fetched = self
            .fetch_with_session(
                "GET",
                url,
                None,
                options,
                super::REQUEST_TIMEOUT_SECS,
                &mut session,
            )
            .await;
        match fetched {
            Ok(capture) => Ok(CapturedFetch {
                capture,
                provenance: session.provenance()?,
            }),
            Err(error) => {
                session.fail(&error)?;
                Err(error)
            }
        }
    }
}

impl From<census_store::CaptureError> for FetchError {
    fn from(value: census_store::CaptureError) -> Self {
        Self::from(census_store::StoreError::from(value))
    }
}

fn accepted(captured: CapturedFetch, options: &FetchOptions) -> Result<CapturedFetch, FetchError> {
    if captured.capture.status == 200 || (captured.capture.status == 404 && options.allow_not_found)
    {
        Ok(captured)
    } else {
        Err(FetchError::Http {
            status: captured.capture.status,
            url: captured.capture.url,
        })
    }
}
