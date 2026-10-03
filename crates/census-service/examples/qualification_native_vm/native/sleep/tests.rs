use super::unfinished;
use serde_json::{json, Value};

fn clock() -> Value {
    json!({"realtime":"2023-11-14T22:13:20Z"})
}

fn observation() -> Value {
    let state = json!({"id":"inv_fixture","target_service_name":"Sweep","target_service_key":"fixture","target_handler_name":"run","status":"suspended","pinned_deployment_id":"deployment_fixture","pinned_service_protocol_version":4,"journal_size":1,"modified_at":"2023-11-14T22:13:19Z","suspended_waiting_future_json":json!({"FirstCompleted":[{"Single":{"CompletionId":7}},{"Single":{"SignalName":"interrupt"}}]}).to_string()});
    let command =
        json!({"Command":{"Sleep":{"wake_up_time":1700000120000_u64,"completion_id":7,"name":""}}});
    json!({"id":"inv_fixture","key":"fixture","before":{"rows":[state.clone()]},"after":{"rows":[state]},"journal":{"rows":[{"id":"inv_fixture","index":0,"version":2,"entry_type":"Command: Sleep","completed":null,"entry_json":command.to_string()}]}})
}

#[test]
fn v2_sleep_requires_positive_suspended_future_not_nullable_legacy_flag() -> anyhow::Result<()> {
    let mut snapshot = observation();
    let expected = Some(
        json!({"completion_id":7,"wake_up_time":1700000120000_u64,"guest_now":1700000000000_u64,"safety_margin_ms":30000}),
    );
    assert_eq!(unfinished(&snapshot, &clock())?, expected);
    snapshot["journal"]["rows"][0]
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("test journal"))?
        .remove("completed");
    assert_eq!(unfinished(&snapshot, &clock())?, expected);
    for side in ["before", "after"] {
        snapshot[side]["rows"][0]["status"] = json!("running");
    }
    assert_eq!(unfinished(&snapshot, &clock())?, None);
    Ok(())
}

#[test]
fn matching_completion_disqualifies_sleep_while_foreign_completion_does_not() -> anyhow::Result<()>
{
    let mut snapshot = observation();
    for side in ["before", "after"] {
        snapshot[side]["rows"][0]["journal_size"] = json!(2);
    }
    let rows = snapshot["journal"]["rows"]
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("test rows"))?;
    rows.push(json!({"id":"inv_fixture","index":1,"version":2,"entry_type":"Notification: Sleep","entry_json":json!({"Notification":{"Completion":{"Sleep":{"completion_id":7}}}}).to_string()}));
    assert_eq!(unfinished(&snapshot, &clock())?, None);
    snapshot["journal"]["rows"][1]["entry_json"] =
        json!(json!({"Notification":{"Completion":{"Sleep":{"completion_id":8}}}}).to_string());
    assert_eq!(
        unfinished(&snapshot, &clock())?.map(|proof| proof["completion_id"].clone()),
        Some(json!(7))
    );
    Ok(())
}

#[test]
fn changed_bookends_incomplete_journal_foreign_target_or_unawaited_sleep_refuse(
) -> anyhow::Result<()> {
    let mut changed = observation();
    changed["after"]["rows"][0]["modified_at"] = json!("2023-11-14T22:13:20Z");
    assert_eq!(unfinished(&changed, &clock())?, None);
    for field in [
        "journal_size",
        "target_service_key",
        "suspended_waiting_future_json",
    ] {
        let mut snapshot = observation();
        for side in ["before", "after"] {
            snapshot[side]["rows"][0][field] = match field {
                "journal_size" => json!(2),
                "target_service_key" => json!("foreign"),
                _ => json!(json!({"Single":{"CompletionId":8}}).to_string()),
            };
        }
        assert_eq!(unfinished(&snapshot, &clock())?, None, "{field}");
    }
    let late = json!({"realtime":"2023-11-14T22:14:51Z"});
    assert_eq!(unfinished(&observation(), &late)?, None);
    Ok(())
}

#[test]
fn malformed_journal_and_future_after_matching_leaf_cannot_certify_boundary() {
    let mut snapshot = observation();
    snapshot["journal"]["rows"][0]["index"] = json!(1);
    assert_eq!(
        unfinished(&snapshot, &clock())
            .err()
            .map(|error| error.to_string()),
        Some("incomplete journal indexes".to_string())
    );
    let mut snapshot = observation();
    let future =
        json!({"FirstCompleted":[{"Broken":[]},{"Single":{"CompletionId":7}}]}).to_string();
    for side in ["before", "after"] {
        snapshot[side]["rows"][0]["suspended_waiting_future_json"] = json!(future);
    }
    assert_eq!(
        unfinished(&snapshot, &clock())
            .err()
            .map(|error| error.to_string()),
        Some("unknown suspended future variant".to_string())
    );
}

fn future(snapshot: &mut Value, tree: Value) {
    for side in ["before", "after"] {
        snapshot[side]["rows"][0]["suspended_waiting_future_json"] = json!(tree.to_string());
    }
}

fn append(snapshot: &mut Value, kind: &str, entry: Value) -> anyhow::Result<()> {
    let rows = snapshot["journal"]["rows"]
        .as_array_mut()
        .ok_or_else(|| anyhow::anyhow!("test rows"))?;
    let index = rows.len();
    rows.push(json!({"id":"inv_fixture","index":index,"version":2,"entry_type":kind,"entry_json":entry.to_string()}));
    let size = rows.len();
    for side in ["before", "after"] {
        snapshot[side]["rows"][0]["journal_size"] = json!(size);
    }
    Ok(())
}

fn refused(snapshot: &Value, expected: &str) {
    assert_eq!(
        unfinished(snapshot, &clock())
            .err()
            .map(|error| error.to_string()),
        Some(expected.to_string())
    );
}

#[test]
fn synthetic_indexed_signals_preserve_exact_sleep_wait() -> anyhow::Result<()> {
    for index in [0_u32, 1, u32::MAX] {
        let mut snapshot = observation();
        future(
            &mut snapshot,
            json!({"FirstCompleted":[
                {"Single":{"CompletionId":7}}, {"Single":{"SignalIndex":index}}
            ]}),
        );
        assert_eq!(
            unfinished(&snapshot, &clock())?.map(|proof| proof["completion_id"].clone()),
            Some(json!(7))
        );
    }
    Ok(())
}

#[test]
fn ambiguous_matching_leaves_and_invalid_notification_ids_refuse() {
    for (tree, error) in [
        (
            json!({"Single":{"CompletionId":7},"Broken":[]}),
            "ambiguous suspended future node",
        ),
        (
            json!({"Single":{"CompletionId":7,"SignalName":"interrupt"}}),
            "ambiguous future notification",
        ),
        (
            json!({"FirstCompleted":[{"Single":{"CompletionId":7}},{"Single":{"SignalId":0}}]}),
            "unknown future notification",
        ),
        (
            json!({"FirstCompleted":[{"Single":{"CompletionId":7}},{"Single":{"SignalIndex":4294967296_u64}}]}),
            "notification index outside u32",
        ),
        (
            json!({"Single":{"CompletionId":4294967296_u64}}),
            "notification index outside u32",
        ),
        (
            json!({"FirstCompleted":[{"Single":{"CompletionId":7}},{"Single":{"SignalName":null}}]}),
            "signal name malformed",
        ),
    ] {
        let mut snapshot = observation();
        future(&mut snapshot, tree);
        refused(&snapshot, error);
    }
}

#[test]
fn future_node_budget_includes_root_and_accepts_exact_limit() -> anyhow::Result<()> {
    for count in [255_usize, 256, 257] {
        let mut snapshot = observation();
        let leaves = (1..count)
            .map(|id| json!({"Single":{"CompletionId":id}}))
            .collect::<Vec<_>>();
        future(&mut snapshot, json!({"FirstCompleted":leaves}));
        if count == 257 {
            refused(
                &snapshot,
                "suspended future traversal exceeds witness budget",
            );
        } else {
            assert_eq!(
                unfinished(&snapshot, &clock())?.map(|proof| proof["completion_id"].clone()),
                Some(json!(7))
            );
        }
    }
    Ok(())
}

#[test]
fn malformed_entry_envelopes_types_and_foreign_completions_refuse() -> anyhow::Result<()> {
    for (kind, entry, error) in [
        (
            "Notification: Sleep",
            json!({"Notification":{}}),
            "notification must contain exactly one variant",
        ),
        (
            "Notification: Sleep",
            json!({"Notification":{"Completion":{"Unknown":{"completion_id":8}}}}),
            "unknown completion variant",
        ),
        (
            "Notification: Run",
            json!({"Notification":{"Completion":{"Sleep":{"completion_id":8}}}}),
            "journal entry type mismatch",
        ),
        (
            "Notification: Sleep",
            json!({"Notification":{"Completion":{"Sleep":{"completion_id":4294967296_u64}}}}),
            "completion ID outside u32",
        ),
        (
            "Notification: Run",
            json!({"Notification":{"Completion":{"Run":{"completion_id":8,"result":{"Success":[256]}}}}}),
            "byte array malformed",
        ),
        (
            "Notification: Run",
            json!({"Notification":{"Completion":{"Run":{"completion_id":8}}}}),
            "required journal field absent: result",
        ),
        (
            "Notification: Sleep",
            json!({"Notification":{"Completion":{"Sleep":{"completion_id":8,"result":"Void"}}}}),
            "unknown journal field: result",
        ),
        (
            "Command: Sleep",
            json!({"Command":{"Sleep":{"completion_id":7,"wake_up_time":1700000120000_u64,"name":""}},"Notification":{}}),
            "entry must contain exactly one variant",
        ),
        (
            "Command: Sleep",
            json!({"Command":{"Sleep":{"completion_id":7,"wake_up_time":1700000120000_u64,"name":""},"Run":{"completion_id":9,"name":""}}}),
            "command must contain exactly one variant",
        ),
    ] {
        let mut snapshot = observation();
        append(&mut snapshot, kind, entry)?;
        refused(&snapshot, error);
    }
    Ok(())
}

#[test]
fn matching_non_sleep_completion_also_disqualifies_and_valid_signal_does_not() -> anyhow::Result<()>
{
    let mut snapshot = observation();
    append(
        &mut snapshot,
        "Notification: Signal",
        json!({"Notification":{"Signal":{
            "id":{"Index":1},"result":"Void"
        }}}),
    )?;
    append(
        &mut snapshot,
        "Notification: Run",
        json!({"Notification":{"Completion":{"Run":{
            "completion_id":8,"result":{"Failure":{"code":500,"message":"failed"}}
        }}}}),
    )?;
    assert_eq!(
        unfinished(&snapshot, &clock())?.map(|proof| proof["completion_id"].clone()),
        Some(json!(7))
    );
    snapshot["journal"]["rows"][2]["entry_json"] =
        json!(json!({"Notification":{"Completion":{"Run":{
            "completion_id":7,"result":{"Success":[42]}
        }}}})
        .to_string());
    assert_eq!(unfinished(&snapshot, &clock())?, None);
    Ok(())
}

mod journal;
mod root06;
