use super::*;

impl PageObserver {
    pub(super) fn note_request(
        progress: &mut WireTransfers,
        event: &EventRequestWillBeSent,
        observation: &mut Observation,
        latest: &mut Option<RequestId>,
    ) {
        *latest = Some(event.request_id.clone());
        progress.bind(&event.request_id);
        *observation = empty_observation(event.request.url.clone());
    }

    pub(super) fn note_response(
        &self,
        latest: &Option<RequestId>,
        event: &EventResponseReceived,
        observation: &mut Observation,
    ) -> Result<(), BrowserError> {
        if !is_current(latest, &event.request_id) {
            return Ok(());
        }
        if latest.is_some() {
            self.record_status(observation, event)?;
        }
        Ok(())
    }

    pub(super) fn note_finished(
        page: &Page,
        progress: &mut WireTransfers,
        latest: &Option<RequestId>,
        event: &EventLoadingFinished,
        observation: &mut Observation,
    ) -> Result<Option<(RequestId, f64)>, BrowserError> {
        if !is_current(latest, &event.request_id) {
            return Ok(None);
        }
        if latest.is_none() {
            return Ok(None);
        }
        let id = event.request_id.clone();
        let total = event.encoded_data_length;
        if let Err(e) = progress.finish(&id, total) {
            tracing::debug!("stream validation failed: {e}");
            observation.failed = true;
            observation.body_complete = true;
            store_observation(page, observation.clone())?;
            return Ok(None);
        }
        Ok(Some((id, total)))
    }

    pub(super) fn note_failure(
        &self,
        latest: &Option<RequestId>,
        event: &EventLoadingFailed,
        observation: &mut Observation,
    ) -> Result<(), BrowserError> {
        if !is_current(latest, &event.request_id) {
            return Ok(());
        }
        if latest.is_some() {
            if event.error_text == REDIRECT_ABORT || event.canceled == Some(true) {
                return Ok(());
            }
            observation.failed = true;
            observation.body_complete = true;
            store_observation(&self.page, observation.clone())?;
        }
        Ok(())
    }

    pub(super) fn note_data(
        page: &Page,
        progress: &mut WireTransfers,
        latest: &Option<RequestId>,
        event: &EventDataReceived,
        observation: &mut Observation,
    ) -> Result<(), BrowserError> {
        if !is_current(latest, &event.request_id) {
            return Ok(());
        }
        if latest.is_some() {
            if let Err(e) = progress.observe(
                &event.request_id,
                event.encoded_data_length,
                event.data_length,
            ) {
                observation.failed = true;
                observation.body_complete = true;
                tracing::debug!("stream observation failed: {e}");
                store_observation(page, observation.clone())?;
            }
        }
        Ok(())
    }

    pub(super) fn note_body(
        &self,
        latest: &Option<RequestId>,
        request_id: &RequestId,
        body_result: Result<Vec<u8>, BrowserError>,
        observation: &mut Observation,
    ) -> Result<(), BrowserError> {
        if latest.as_ref() != Some(request_id) {
            return Ok(());
        }
        match body_result {
            Ok(body) => self.record_body(observation, &body),
            Err(e) => {
                tracing::debug!("body capture failed: {e}");
                Ok(())
            }
        }
    }
}
