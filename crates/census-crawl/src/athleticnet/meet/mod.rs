
mod collect;
mod count;
mod map;
mod read;
mod store;
#[cfg(test)]
mod tests;
mod wire;

pub(in crate::athleticnet) use collect::collect as collect_meets;
pub(in crate::athleticnet) use map::absorb_meet;
pub use read::{grade_of, jurisdiction_of, EventMetadata};
pub use wire::{
    AllResults, EventDivisions, FlatEvent, FlatRow, MeetData, PublishedEvent, PublishedLeg,
    PublishedMeet, PublishedTeam,
};

pub(super) const MEET_ENDPOINT: &str = "https://www.athletic.net/api/v1/Meet/GetMeetData";

pub(super) const RESULTS_ENDPOINT: &str = "https://www.athletic.net/api/v1/Meet/GetAllResultsData";

pub(super) const METADATA_ENDPOINT: &str =
    "https://www.athletic.net/api/v1/Meet/GetEventDivisionData";

pub fn meet_requests(meet_id: i64) -> [String; 2] {
    [
        format!("{MEET_ENDPOINT}?meetId={meet_id}&sport=tf"),
        format!("{RESULTS_ENDPOINT}?meetId={meet_id}&sport=tf&rawResults=false&showTips=false"),
    ]
}

pub fn metadata_request(meet_id: i64) -> String {
    format!("{METADATA_ENDPOINT}?meetId={meet_id}&sport=tf")
}
