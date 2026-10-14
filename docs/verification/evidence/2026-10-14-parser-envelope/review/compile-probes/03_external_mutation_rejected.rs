use census_crawl::milesplit::{OwnedMeetOutcome, OwnedMeetVerdict};
use census_crawl::net::CapturedFetch;

pub fn mutate_capture(value: &mut CapturedFetch) { value.capture().body.clear(); }
pub fn mutate_interpreted_capture(value: &mut OwnedMeetOutcome) { value.capture().body.clear(); }
pub fn mutate_verdict(value: &mut OwnedMeetOutcome) {
    if let OwnedMeetVerdict::Parsed(page) = value.verdict() { page.rows.clear(); }
}
pub fn release_outside_owner(value: &mut OwnedMeetOutcome) { value.release_body(); }
fn main() {}
