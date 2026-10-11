use super::*;

#[test]
fn epoch_formats_as_utc() {
    assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
    assert_eq!(rfc3339(1_000_000_000), "2001-09-09T01:46:40Z");
    assert_eq!(rfc3339(1_600_000_000), "2020-09-13T12:26:40Z");
}

#[test]
fn leap_day_formats() {
    assert_eq!(rfc3339(1_582_934_400), "2020-02-29T00:00:00Z");
}

#[test]
fn the_admin_query_endpoint_joins_once() {
    assert_eq!(
        admin_query_endpoint("http://127.0.0.1:19095/"),
        "http://127.0.0.1:19095/query"
    );
    assert_eq!(
        admin_query_endpoint("http://127.0.0.1:19095"),
        "http://127.0.0.1:19095/query"
    );
}

#[test]
fn the_written_block_round_trips_through_the_review_parser() {
    let summary = json!({
        "sampled_at": "2026-10-09T16:11:17Z",
        "season": "2026-27",
        "revision": 1,
        "jurisdiction_sweeps": 49,
        "jurisdiction_total": 49,
        "source_objects_owed": 63605,
        "silent_sources": 0,
        "unreadable": 0,
        "invocations": Value::Null,
        "owed_sets": [
            {"count": 7, "stages": "rosters, meets_history, results_history"},
            {"count": 40, "stages": "teams, rosters, meets_history"}
        ]
    });
    let block = block_text(&summary);
    assert!(!block.contains("null"));
    assert!(!block.contains('"'));
    let parsed =
        crate::review::build::latest_sample(&block).map_or(Value::Null, core::convert::identity);
    assert_eq!(
        parsed.get("sampled_at"),
        Some(&json!("2026-10-09T16:11:17Z"))
    );
    assert_eq!(parsed.get("sweeps_owed"), Some(&json!(49)));
    assert_eq!(parsed.get("sweeps_total"), Some(&json!(49)));
    assert_eq!(parsed.get("source_objects_owed"), Some(&json!(63605)));
    assert_eq!(parsed.get("invocations"), Some(&Value::Null));
    assert_eq!(parsed.pointer("/owed_sets/0/count"), Some(&json!(7)));
    assert_eq!(parsed.pointer("/owed_sets/1/count"), Some(&json!(40)));
    assert_eq!(
        parsed.pointer("/owed_sets/1/stages"),
        Some(&json!("teams, rosters, meets_history"))
    );
}
