use super::{accepted, read, CaptureSession, CapturedFetch};
use crate::net::{CaptureTransportError, FetchError, FetchOptions, FetchOutcome, Fetcher};
use census_domain::model::{AcquisitionWorkV1, CaptureCacheIntent, InterruptionCauseV1};
use census_store::{AcquireDecision, AcquisitionReadBudget, OwnedAcquisitionWork, Store};

impl Fetcher {
    #[tracing::instrument(skip(self, store, work, options))]
    pub(crate) async fn get_acquisition(
        &self,
        store: &Store,
        work: OwnedAcquisitionWork,
        options: &FetchOptions,
    ) -> Result<CapturedFetch, FetchError> {
        self.get_acquisition_with_scope(
            store,
            work,
            options,
            crate::net::public_http::TransportScope::Provider,
        )
        .await
    }

    pub(super) async fn get_acquisition_with_scope(
        &self,
        store: &Store,
        work: OwnedAcquisitionWork,
        options: &FetchOptions,
        scope: crate::net::public_http::TransportScope<'_>,
    ) -> Result<CapturedFetch, FetchError> {
        if work
            .work()
            .slot()
            .public_resource()
            .is_some_and(|resource| {
                resource.kind() == census_domain::model::PublicSourceResourceKindV1::Robots
            })
        {
            return Err(FetchError::Policy {
                detail: "robots.txt dispatch is disabled".to_owned(),
            });
        }
        let url = work.work().request().original_url().to_owned();
        scope.check(&url)?;
        let options = self.checked_acquisition_options(work.work(), options)?;
        let options = options.as_ref();
        let mut budget = AcquisitionReadBudget::for_work();
        let captured = match store.inspect_acquisition_work(&work, &mut budget)? {
            AcquireDecision::Replay(reference) => {
                accepted(read::replay(store, &reference)?, options)
            }
            AcquireDecision::FinishPrepared(_) => {
                if work.protocol() == census_domain::model::CaptureProtocolVersion::V2 {
                    accepted(
                        self.recover_prepared_acquisition(
                            store,
                            work,
                            census_store::ExactBodySource::NoneSupplied,
                        )?,
                        options,
                    )
                } else {
                    accepted(self.recover_prepared(store, work)?, options)
                }
            }
            AcquireDecision::RecoveryRequired(_) => {
                store.recover_acquisition_attempt(work, InterruptionCauseV1::OwnerRestarted)?;
                Err(census_store::CaptureError::RetainedOutcomeUnknown.into())
            }
            AcquireDecision::Ready
            | AcquireDecision::ResumeReserved(_)
            | AcquireDecision::RetryOwed(_) => {
                self.acquire_registered(store, work, &url, options, scope)
                    .await
            }
            AcquireDecision::Settled(_) => Err(CaptureTransportError::Settled.into()),
            AcquireDecision::Exhausted => {
                Err(census_store::CaptureError::AcquisitionBudgetExhausted.into())
            }
            AcquireDecision::Interrupted => Err(CaptureTransportError::Interrupted.into()),
        }?;
        scope.check(
            captured
                .capture()
                .response_url
                .as_deref()
                .map_or(captured.capture().url.as_str(), |url| url),
        )?;
        Ok(captured)
    }

    pub(super) fn checked_acquisition_options<'a>(
        &self,
        work: &AcquisitionWorkV1,
        options: &'a FetchOptions,
    ) -> Result<std::borrow::Cow<'a, FetchOptions>, FetchError> {
        let observed = work.request().observed_http_required();
        let options = if observed {
            std::borrow::Cow::Owned(self.observed_options(options)?)
        } else {
            std::borrow::Cow::Borrowed(options)
        };
        let url = work.request().original_url();
        let request = if observed {
            self.public_resource_request(url, &options)?
        } else {
            self.acquisition_request(url, &options)?
        };
        let intent = if options.refresh {
            CaptureCacheIntent::AcquireNew
        } else {
            CaptureCacheIntent::ReuseEligible
        };
        if request != *work.request() || intent != work.cache_intent() {
            return Err(CaptureTransportError::RequestMismatch.into());
        }
        Ok(options)
    }

    #[tracing::instrument(skip(self, store, work, options, scope))]
    async fn acquire_registered(
        &self,
        store: &Store,
        work: OwnedAcquisitionWork,
        url: &str,
        options: &FetchOptions,
        scope: crate::net::public_http::TransportScope<'_>,
    ) -> Result<CapturedFetch, FetchError> {
        let representation =
            super::super::execute::representation::canonical_request(url, &options.headers)?;
        let extra = super::super::execute::representation::cache_extra(None, &representation);
        let (body_path, meta_path) = self.cache_paths(&Self::key_for("GET", url, &extra));
        let cached =
            super::super::cache::read_eligible(&body_path, &meta_path, url, &representation)?;
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
            .fetch_with_scope(
                "GET",
                url,
                None,
                options,
                super::super::REQUEST_TIMEOUT_SECS,
                &mut session,
                scope,
            )
            .await;
        finish_acquisition(session, fetched)
    }
}

fn finish_acquisition(
    mut session: CaptureSession<'_>,
    fetched: Result<FetchOutcome, FetchError>,
) -> Result<CapturedFetch, FetchError> {
    match fetched {
        Ok(capture) => {
            let (provenance, original_metadata) = session.into_parts()?;
            Ok(CapturedFetch {
                capture,
                provenance,
                original_metadata,
            })
        }
        Err(error) => {
            session.fail(&error)?;
            Err(error)
        }
    }
}
