use crate::milesplit::{OwnedMeetOutcome, OwnedMeetVerdict};
use crate::net::FetchOutcome;

pub fn fabricate(raw: FetchOutcome, verdict: OwnedMeetVerdict) -> OwnedMeetOutcome {
    OwnedMeetOutcome::historical_fixture(raw, verdict)
}
