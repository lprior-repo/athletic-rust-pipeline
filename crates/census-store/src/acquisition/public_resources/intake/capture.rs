use super::{codec, refused, MAX_BODY};
use crate::acquisition::{backing, facts::*, observed, read, CaptureError};
use crate::{Receipt, StoreResult};
use census_domain::model::*;
use serde::{Deserialize, Serialize};

mod body;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublicSchoolIntakeCaptureV1 {
    pub(super) work: AcquisitionWorkV1,
    pub(super) prepared: Prepared,
    pub(super) terminal: ImmutableAttemptResult,
    pub(super) terminal_receipt: Receipt,
    pub(super) result: WorkResult,
    pub(super) response: ResponseObservationV1,
    pub(super) occurrence: AcquisitionOccurrenceRefV1,
    #[serde(with = "body")]
    pub(super) body: Vec<u8>,
}

impl PublicSchoolIntakeCaptureV1 {
    pub fn work(&self) -> &AcquisitionWorkV1 {
        &self.work
    }
    pub fn prepared(&self) -> &PreparedCaptureInputV1 {
        &self.prepared.capture
    }
    pub fn body(&self) -> &[u8] {
        &self.body
    }
    pub fn occurrence(&self) -> &AcquisitionOccurrenceRefV1 {
        &self.occurrence
    }
    pub fn body_reference(&self) -> &AcquisitionOccurrenceRefV1 {
        &self.occurrence
    }
    pub fn response(&self) -> &ResponseObservationV1 {
        &self.response
    }

    pub(super) fn validate(&self) -> StoreResult<()> {
        self.validate_identity()?;
        self.validate_terminal()?;
        if self.body.is_empty()
            || self.body.len() > MAX_BODY
            || u64::try_from(self.body.len()).map_err(|_| crate::StoreError::CounterOverflow)?
                != self.prepared.capture.content_bytes()
            || codec::digest(&self.body)? != *self.prepared.capture.content_digest()
            || std::str::from_utf8(&self.body).is_err()
        {
            return Err(refused("public school intake original body differs"));
        }
        let facts = self.validation_facts()?;
        observed::prefix(&self.work, &facts, None, None)?;
        backing::validate_metadata(&self.work, &facts, &self.prepared.capture)
    }

    fn validate_identity(&self) -> StoreResult<()> {
        if self.prepared.version != CaptureProtocolVersion::V2
            || self.terminal.protocol() != CaptureProtocolVersion::V2
            || self.result.version != CaptureProtocolVersion::V2
            || !self.work.request().observed_http_required()
            || self.prepared.attempt != *self.terminal.attempt()
            || self.prepared.dispatch.attempt() != self.terminal.attempt()
            || self.terminal.occurrence()? != self.occurrence
            || self.result.occurrence != self.occurrence
            || self.occurrence.attempt().work_id()
                != &self.work.id().map_err(CaptureError::Domain)?
            || self.occurrence.attempt().store_identity() != self.work.slot().store_identity()
            || self.response.status() != 200
            || self.response.response_url() != self.work.request().original_url()
        {
            return Err(refused("public school intake original identity differs"));
        }
        Ok(())
    }

    fn validate_terminal(&self) -> StoreResult<()> {
        if !matches!(self.terminal.terminal(), AcquisitionTerminal::ResponseRetained {
            prepared_digest, disposition: CaptureDispositionV1::ReusableResponse
        } if prepared_digest == &digest(&self.prepared)?)
            || self.prepared.disposition != CaptureDispositionV1::ReusableResponse
            || self.terminal_receipt.operation
                != read::terminal_operation(self.terminal.attempt(), CaptureProtocolVersion::V2)
            || self.terminal_receipt.digest != digest(&self.terminal)?.as_str()
            || self.terminal_receipt.appended != 0
            || self.terminal_receipt.credited
        {
            return Err(refused("public school intake terminal proof differs"));
        }
        let at = super::terminal_day(&self.terminal_receipt)?;
        let response = chrono::DateTime::parse_from_rfc3339(self.response.observed_at())
            .map_err(|_| refused("public school intake observed time"))?;
        if at < response.with_timezone(&chrono::Utc).date_naive() {
            return Err(refused("public school intake terminal precedes response"));
        }
        Ok(())
    }

    fn validation_facts(&self) -> StoreResult<AttemptFacts> {
        #[derive(Deserialize)]
        struct Metadata {
            observed_http: NormalizedObservedHttpV1,
        }
        let metadata: Metadata = serde_json::from_slice(self.prepared.capture.metadata_bytes())
            .map_err(|_| refused("public school intake observed metadata"))?;
        let [hop] = metadata.observed_http.hops() else {
            return Err(refused("public school intake requires one original hop"));
        };
        if hop.dispatch() != &self.prepared.dispatch
            || hop.dispatch().hop().get() != 0
            || Some(hop.response()) != self.response.observed_http()
            || hop.request().target_url() != self.work.request().original_url()
        {
            return Err(refused("public school intake observed original differs"));
        }
        require_headers(hop.response())?;
        let request = DispatchRequestV1::new_observed(
            self.work.request().original_url(),
            ConditionalRequestV1::Unconditional,
            hop.request().clone(),
        )
        .map_err(CaptureError::Domain)?;
        observed::request(&self.work, &request)?;
        observed::response(
            &self.work,
            &self.prepared.dispatch,
            &request,
            &self.response,
        )?;
        Ok(AttemptFacts {
            reservation: Reservation {
                version: CaptureProtocolVersion::V2,
                attempt: self.terminal.attempt().clone(),
            },
            dispatches: vec![(
                Armed {
                    version: CaptureProtocolVersion::V2,
                    dispatch: self.prepared.dispatch.clone(),
                    request,
                },
                Some(Observed {
                    version: CaptureProtocolVersion::V2,
                    dispatch: self.prepared.dispatch.clone(),
                    response: self.response.clone(),
                }),
            )],
            prepared: Some(self.prepared.clone()),
            terminal: Some(self.terminal.clone()),
            receipt: ReceiptWitness::Present {
                receipt: self.terminal_receipt.clone(),
                raw: codec::encode(&self.terminal_receipt)?,
            },
        })
    }
}

fn require_headers(response: &ObservedHttpResponseV1) -> StoreResult<()> {
    let mut mime = response
        .headers()
        .iter()
        .filter(|header| header.name() == "content-type");
    let value = mime.next().map(|header| header.value());
    if mime.next().is_some()
        || !value.is_some_and(|value| {
            value.eq_ignore_ascii_case(b"text/html")
                || value.eq_ignore_ascii_case(b"text/html; charset=utf-8")
        })
        || response
            .headers()
            .iter()
            .any(|header| header.name() == "location")
    {
        return Err(refused("public school intake HTML headers differ"));
    }
    Ok(())
}
