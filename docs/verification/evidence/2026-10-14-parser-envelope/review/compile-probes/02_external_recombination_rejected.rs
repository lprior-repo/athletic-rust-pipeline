use census_crawl::milesplit::{OwnedMeetOutcome, OwnedMeetVerdict};
use census_crawl::net::{CapturedFetch, FetchOutcome};

pub fn recombine_capture(value: CapturedFetch, unrelated: FetchOutcome) -> CapturedFetch {
    let CapturedFetch { capture: _, provenance } = value;
    CapturedFetch { capture: unrelated, provenance }
}

pub fn recombine_verdict(value: OwnedMeetOutcome, unrelated: OwnedMeetVerdict) -> OwnedMeetOutcome {
    let OwnedMeetOutcome { capture, provenance, verdict: _ } = value;
    OwnedMeetOutcome { capture, provenance, verdict: unrelated }
}
fn main() {}
