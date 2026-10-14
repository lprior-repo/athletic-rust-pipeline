use census_crawl::milesplit::OwnedMeetOutcome;
use census_crawl::net::{CapturedFetch, FetchOutcome};

pub fn decode_capture(raw: &str) -> Result<CapturedFetch, serde_json::Error> { serde_json::from_str(raw) }
pub fn decode_outcome(raw: &str) -> Result<OwnedMeetOutcome, serde_json::Error> { serde_json::from_str(raw) }
pub fn default_capture() -> CapturedFetch { Default::default() }
pub fn default_outcome() -> OwnedMeetOutcome { Default::default() }
pub fn promote_capture(raw: FetchOutcome) -> CapturedFetch { raw.into() }
pub fn promote_outcome(raw: FetchOutcome) -> OwnedMeetOutcome { raw.into() }
fn main() {}
