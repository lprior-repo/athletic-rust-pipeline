pub const BLOB_ORIGIN: &str = "https://athleticlive.blob.core.windows.net";

pub const RTDB_ORIGIN: &str = "https://s-gke-usc1-nssi3-33.firebaseio.com";

pub const RTDB_NAMESPACE: &str = "trackmeet-io";

pub const EVENT_INDEX: &str = "ind_res_list";

pub fn event_doc_url(event_id: u64) -> String {
    format!("{BLOB_ORIGIN}/$web/{EVENT_INDEX}/_doc/{event_id}")
}

pub fn event_summary_url(meet_id: u64) -> String {
    format!("{RTDB_ORIGIN}/meet_{meet_id}/event_summary.json?ns={RTDB_NAMESPACE}")
}

pub fn standings_url(meet_id: u64, run_id: &str) -> Option<String> {
    valid_run_id(run_id).then(|| {
        format!("{RTDB_ORIGIN}/meet_{meet_id}/liveRunStandings/{run_id}.json?ns={RTDB_NAMESPACE}")
    })
}

fn valid_run_id(run_id: &str) -> bool {
    !run_id.is_empty()
        && run_id.len() <= 16
        && run_id.chars().all(|c| c.is_ascii_digit() || c == '-')
        && run_id.split('-').all(|part| !part.is_empty())
}
