use census_crawl::milesplit::{parse_owned_meet, OwnedMeetOutcome, OwnedMeetPage, OwnedMeetParseError, OwnedMeetVerdict};
use census_crawl::net::{CapturedFetch, CaptureProvenanceView, FetchOutcome};

pub fn inspect_capture(value: &CapturedFetch) -> (&FetchOutcome, CaptureProvenanceView<'_>) {
    (value.capture(), value.provenance())
}

pub fn inspect_outcome(value: &OwnedMeetOutcome) -> (&FetchOutcome, &OwnedMeetVerdict, CaptureProvenanceView<'_>) {
    (value.capture(), value.verdict(), value.provenance())
}

pub fn erase_capture(value: CapturedFetch) -> FetchOutcome { value.into_capture() }
pub fn erase_outcome(value: OwnedMeetOutcome) -> FetchOutcome { value.into_capture() }
pub fn parse_raw(bytes: &[u8]) -> Result<OwnedMeetPage, OwnedMeetParseError> { parse_owned_meet(bytes, 725218) }
fn main() {}
