use super::*;
use census_domain::model::{
    AcquisitionDigest, NormalizedObservedHttpV1, ObservedHttpHopV1, ResponseObservationV1,
};

impl CaptureSession<'_> {
    pub(in crate::net) fn observed_http_required(&self) -> bool {
        matches!(self, Self::Adopted(adopted) if adopted.observed_http.is_some())
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
        let result = self.response_observation(response, observed_at);
        let observed = match result {
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
        let hop = self.prepare_hop(&observation, &observed)?;
        let retained = store.record_acquisition_response(*observation, observed)?;
        let Self::Adopted(adopted) = self else {
            return Err(CaptureTransportError::DispatchConsumed.into());
        };
        if let (Some(hops), Some(hop)) = (&mut adopted.observed_http, hop) {
            hops.push(hop);
        }
        adopted.stage = Stage::Observed(retained);
        Ok(())
    }

    fn response_observation(
        &self,
        response: &reqwest::Response,
        observed_at: &str,
    ) -> Result<ResponseObservationV1, FetchError> {
        let result = observation_of(response, observed_at)?;
        if !self.observed_http_required() {
            return Ok(result);
        }
        Ok(ResponseObservationV1::new_observed(
            result.status(),
            result.response_url(),
            result.observed_at(),
            result.headers().to_vec(),
            super::super::normalized::response(response, observed_at)?,
        )?)
    }

    fn prepare_hop(
        &mut self,
        dispatch: &DispatchObservation,
        response: &ResponseObservationV1,
    ) -> Result<Option<ObservedHttpHopV1>, FetchError> {
        let Self::Adopted(adopted) = self else {
            return Ok(None);
        };
        let Some(hops) = &mut adopted.observed_http else {
            return Ok(None);
        };
        if hops.len() >= 5 {
            return Err(CaptureTransportError::DispatchConsumed.into());
        }
        hops.try_reserve_exact(1)
            .map_err(|_| CaptureTransportError::BackingUnavailable)?;
        let request = dispatch
            .request()
            .observed_http()
            .ok_or(CaptureTransportError::RequestMismatch)?;
        let response = response
            .observed_http()
            .ok_or(CaptureTransportError::RequestMismatch)?;
        Ok(Some(ObservedHttpHopV1::new(
            dispatch.dispatch_id().clone(),
            request.clone(),
            response.clone(),
        )?))
    }

    pub(in crate::net) fn finish_observed_http(
        &self,
        digest: &str,
        bytes: usize,
        completed_at: &str,
    ) -> Result<Option<NormalizedObservedHttpV1>, FetchError> {
        let Self::Adopted(adopted) = self else {
            return Ok(None);
        };
        let Some(hops) = &adopted.observed_http else {
            return Ok(None);
        };
        Ok(Some(NormalizedObservedHttpV1::new(
            hops.clone(),
            completed_at,
            AcquisitionDigest::parse(digest)?,
            u64::try_from(bytes).map_err(|_| CaptureTransportError::BackingUnavailable)?,
        )?))
    }
}
