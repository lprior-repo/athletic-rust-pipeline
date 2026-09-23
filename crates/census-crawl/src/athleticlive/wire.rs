//! The result-plane routes this adapter requests, with the measured unit behind each one.
//!
//! Three anonymous GETs cover the whole result plane. All three are keyed by ids the meet-harvest
//! CSV already carries (`athleticlive_meet_id`, per `[sources/national-aggregators]` §3.5), so the
//! only discovery request is the per-meet event summary:
//!
//! | route | URL | measured capacity | capture |
//! |---|---|---|---|
//! | event document | `https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/<eventId>` | 1 request = one event's worth of rows | `ind_res_list/_doc/2254285` 200, 30,436 B, 43 rows; `_doc/2254280` 200, 17,175 B, 17 rows; `_doc/2150205` 200, 159,531 B, 136 rows |
//! | event summary | `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_<meetId>/event_summary.json?ns=trackmeet-io` | 1 request = every event id of one meet | `meet_61710/event_summary.json` 200, 19,932 B, 32 events (26 individual, 6 relay) |
//! | live standings | `https://s-gke-usc1-nssi3-33.firebaseio.com/meet_<meetId>/liveRunStandings/<runId>.json?ns=trackmeet-io` | 1 request = one race's finishing order | `meet_55421/liveRunStandings/4-1.json` 200, 234,753 B, 186 entries |
//!
//! The blob container also serves `meet_<meetId>/event_summary.json`, but that path 404s (215 B)
//! while the RTDB one answers 200 with the same document name — the summary route is the RTDB one.
//!
//! Request costs that follow from the captures, per meet: **1 summary + 1 per individual event**.
//! Meet 61710 lists 32 events, 26 of them individual, so a full walk of it costs 27 requests and
//! yields 17..43 rows per document (measured above); a meet whose event document is missing costs
//! 1 standings request instead.
//!
//! `[sources/timing-providers-national]` §3 and `[sources/national-aggregators]` §3.13/§3.15 carry
//! the same URLs; every count above is from the captured bytes, not from the reports.

/// The platform's own static-site container; event documents live in the `$web` container.
pub const BLOB_ORIGIN: &str = "https://athleticlive.blob.core.windows.net";

/// The Firebase Realtime Database instance the platform's live results read from.
pub const RTDB_ORIGIN: &str = "https://s-gke-usc1-nssi3-33.firebaseio.com";

/// The tenant namespace the platform's own client requests (`?ns=`).
pub const RTDB_NAMESPACE: &str = "trackmeet-io";

/// The index collection an event document is stored under.
pub const EVENT_INDEX: &str = "ind_res_list";

/// The event document for one AthleticLIVE event id.
pub fn event_doc_url(event_id: u64) -> String {
    format!("{BLOB_ORIGIN}/$web/{EVENT_INDEX}/_doc/{event_id}")
}

/// Every event of one AthleticLIVE meet, keyed by the meet id the harvest CSV publishes.
pub fn event_summary_url(meet_id: u64) -> String {
    format!("{RTDB_ORIGIN}/meet_{meet_id}/event_summary.json?ns={RTDB_NAMESPACE}")
}

/// One race's live standings, or `None` when the published run id is not a path segment.
///
/// The run id arrives in the event summary's own payload (`rui`: `19-1`, `4-1`), so it is external
/// input: the guard keeps a surprising value from rewriting the request path.
pub fn standings_url(meet_id: u64, run_id: &str) -> Option<String> {
    valid_run_id(run_id).then(|| {
        format!("{RTDB_ORIGIN}/meet_{meet_id}/liveRunStandings/{run_id}.json?ns={RTDB_NAMESPACE}")
    })
}

/// A published run id: digits joined by single dashes, as every capture publishes it (`19-1`).
fn valid_run_id(run_id: &str) -> bool {
    !run_id.is_empty()
        && run_id.len() <= 16
        && run_id.chars().all(|c| c.is_ascii_digit() || c == '-')
        && run_id.split('-').all(|part| !part.is_empty())
}
