use census_crawl::milesplit::{OwnedMeetOutcome, OwnedMeetVerdict};
use census_crawl::net::{CapturedFetch, FetchOutcome};

pub fn extract_parts(value: CapturedFetch) { let _parts = value.into_parser_parts(); }
pub fn construct_historical(raw: FetchOutcome) -> CapturedFetch { CapturedFetch::historical(raw) }
pub fn fabricate_interpretation(raw: FetchOutcome, verdict: OwnedMeetVerdict) -> OwnedMeetOutcome {
    OwnedMeetOutcome::historical_fixture(raw, verdict)
}
fn main() {}
