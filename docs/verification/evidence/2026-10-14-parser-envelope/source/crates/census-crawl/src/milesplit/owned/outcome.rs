use super::{parse_owned_meet, OwnedMeetParseError, OwnedMeetVerdict, OwnedParseResourceFailure};
use crate::net::{CaptureProvenance, CaptureProvenanceView, CapturedFetch, FetchOutcome};
use crate::{CaptureContext, CrawlResult};

#[derive(Debug)]
pub struct OwnedMeetOutcome {
    capture: FetchOutcome,
    provenance: CaptureProvenance,
    verdict: OwnedMeetVerdict,
}

#[derive(Debug)]
pub(in crate::milesplit) enum OwnedInterpretationOutcome {
    Interpreted(OwnedMeetOutcome),
    Resource {
        capture: CapturedFetch,
        failure: OwnedParseResourceFailure,
    },
}

impl OwnedMeetOutcome {
    pub(in crate::milesplit) fn interpret(
        acquired: CapturedFetch,
        meet_id: u64,
        context: CaptureContext<'_>,
    ) -> CrawlResult<OwnedInterpretationOutcome> {
        require_capture_context(context, acquired.provenance())?;
        let capture = acquired.capture();
        let verdict = if capture.status == 200 {
            match parse_owned_meet(&capture.body, meet_id) {
                Ok(page) => OwnedMeetVerdict::Parsed(page),
                Err(OwnedMeetParseError::Malformed(failure)) => OwnedMeetVerdict::Malformed {
                    detail: failure.to_string(),
                },
                Err(OwnedMeetParseError::Resource(failure)) => {
                    return Ok(OwnedInterpretationOutcome::Resource {
                        capture: acquired,
                        failure,
                    });
                }
            }
        } else {
            OwnedMeetVerdict::Refused {
                status: capture.status,
            }
        };
        let (capture, provenance) = acquired.into_parser_parts();
        Ok(OwnedInterpretationOutcome::Interpreted(Self {
            capture,
            provenance,
            verdict,
        }))
    }

    pub fn capture(&self) -> &FetchOutcome {
        &self.capture
    }

    pub fn provenance(&self) -> CaptureProvenanceView<'_> {
        self.provenance.view()
    }

    pub fn verdict(&self) -> &OwnedMeetVerdict {
        &self.verdict
    }

    pub fn into_capture(self) -> FetchOutcome {
        self.capture
    }

    pub(in crate::milesplit) fn require_capture_context(
        &self,
        context: CaptureContext<'_>,
    ) -> CrawlResult<()> {
        require_capture_context(context, self.provenance())
    }

    pub(in crate::milesplit) fn release_body(&mut self) {
        self.capture.body = Vec::new();
    }

    #[cfg(test)]
    pub(in crate::milesplit) fn historical_fixture(
        capture: FetchOutcome,
        verdict: OwnedMeetVerdict,
    ) -> Self {
        let (capture, provenance) = CapturedFetch::historical(capture).into_parser_parts();
        Self {
            capture,
            provenance,
            verdict,
        }
    }
}

fn require_capture_context(
    context: CaptureContext<'_>,
    provenance: CaptureProvenanceView<'_>,
) -> CrawlResult<()> {
    match (context, provenance) {
        (CaptureContext::LegacyMetadata, CaptureProvenanceView::HistoricalAttemptUnknown)
        | (CaptureContext::DurableV1(_), CaptureProvenanceView::Retained { .. }) => Ok(()),
        _ => Err(crate::net::CaptureTransportError::CacheOccurrenceMismatch.into()),
    }
}

#[cfg(test)]
mod tests;
