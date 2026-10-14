use crate::milesplit::{OwnedMeetOutcome, OwnedMeetVerdict};
use crate::net::{CapturedFetch, FetchOutcome};
use census_domain::model::AcquisitionOccurrenceRefV1;

pub fn recombine_capture(mut value: CapturedFetch, unrelated: FetchOutcome) -> CapturedFetch {
    value.capture = unrelated;
    value
}
pub fn recombine_outcome(mut value: OwnedMeetOutcome, unrelated: OwnedMeetVerdict) -> OwnedMeetOutcome {
    value.verdict = unrelated;
    value
}
pub fn mint_retained(occurrence: AcquisitionOccurrenceRefV1, original: AcquisitionOccurrenceRefV1) {
    let _provenance = crate::net::CaptureProvenance::retained(occurrence, original);
}
