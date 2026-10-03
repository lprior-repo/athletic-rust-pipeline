use super::{append, clock, future, observation, refused, unfinished};
use serde_json::{json, Value};

#[test]
fn running_and_stale_inflight_future_never_certify_durable_suspension() -> anyhow::Result<()> {
    let mut snapshot = observation();
    for side in ["before", "after"] {
        let state = &mut snapshot[side]["rows"][0];
        state["status"] = json!("running");
        state["last_awaiting_on_future_json"] = state["suspended_waiting_future_json"].clone();
        state["suspended_waiting_future_json"] = Value::Null;
        state["in_flight"] = json!(true);
    }
    assert_eq!(unfinished(&snapshot, &clock())?, None);
    Ok(())
}

#[test]
fn full_input_run_and_sleep_journal_preserves_exact_completion_identity() -> anyhow::Result<()> {
    let mut snapshot = observation();
    snapshot["journal"]["rows"] = json!([]);
    append(
        &mut snapshot,
        "Command: Input",
        json!({"Command":{"Input":{
            "headers":[{"name":"content-type","value":"application/json"}],
            "payload":[123,125],"name":""
        }}}),
    )?;
    append(
        &mut snapshot,
        "Command: Run",
        json!({"Command":{"Run":{"completion_id":1,"name":""}}}),
    )?;
    append(
        &mut snapshot,
        "Notification: Run",
        json!({"Notification":{"Completion":{"Run":{
            "completion_id":1,"result":{"Success":[34,50,48,50,54,34]}
        }}}}),
    )?;
    append(
        &mut snapshot,
        "Command: Sleep",
        json!({"Command":{"Sleep":{
            "completion_id":2,"wake_up_time":1700000120000_u64,"name":""
        }}}),
    )?;
    future(
        &mut snapshot,
        json!({"FirstCompleted":[
            {"Single":{"CompletionId":2}},{"Single":{"SignalIndex":1}}
        ]}),
    );
    assert_eq!(
        unfinished(&snapshot, &clock())?.map(|proof| proof["completion_id"].clone()),
        Some(json!(2))
    );
    snapshot["journal"]["rows"][2]["index"] = json!(3);
    refused(&snapshot, "incomplete journal indexes");
    Ok(())
}

#[test]
fn every_typed_completion_kind_correlates_by_id_not_by_journal_index() -> anyhow::Result<()> {
    for (kind, mut payload) in [
        ("GetLazyState", json!({"result":"Void"})),
        ("GetLazyStateKeys", json!({"state_keys":["a","b"]})),
        ("GetPromise", json!({"result":{"Success":[42]}})),
        ("PeekPromise", json!({"result":"Void"})),
        ("CompletePromise", json!({"result":"Void"})),
        ("Sleep", json!({})),
        ("CallInvocationId", json!({"invocation_id":"inv_fixture"})),
        ("Call", json!({"result":{"Success":[]}})),
        ("Run", json!({"result":{"Success":[42]}})),
        (
            "AttachInvocation",
            json!({"result":{"Failure":{"code":500,"message":"failed","metadata":[{"key":"reason","value":"temporary"}]}}}),
        ),
        ("GetInvocationOutput", json!({"result":"Void"})),
    ] {
        let mut snapshot = observation();
        payload["completion_id"] = json!(8);
        append(
            &mut snapshot,
            &format!("Notification: {kind}"),
            json!({"Notification":{"Completion":{kind:payload.clone()}}}),
        )?;
        assert_eq!(
            unfinished(&snapshot, &clock())?.map(|proof| proof["completion_id"].clone()),
            Some(json!(7)),
            "{kind}"
        );
        payload["completion_id"] = json!(7);
        snapshot["journal"]["rows"][1]["entry_json"] =
            json!(json!({"Notification":{"Completion":{kind:payload}}}).to_string());
        assert_eq!(unfinished(&snapshot, &clock())?, None, "{kind}");
    }
    Ok(())
}

#[test]
fn malformed_result_and_signal_shapes_cannot_be_ignored_after_valid_sleep() -> anyhow::Result<()> {
    for (kind, entry, error) in [
        (
            "Notification: Signal",
            json!({"Notification":{"Signal":{"id":{"Index":4294967296_u64},"result":"Void"}}}),
            "signal index outside u32",
        ),
        (
            "Notification: Signal",
            json!({"Notification":{"Signal":{"id":{"Name":"interrupt","Index":1},"result":"Void"}}}),
            "signal ID must contain exactly one variant",
        ),
        (
            "Notification: Signal",
            json!({"Notification":{"Signal":{"id":{"Name":"interrupt"},"result":{"Unknown":null}}}}),
            "result variant not allowed",
        ),
        (
            "Notification: Run",
            json!({"Notification":{"Completion":{"Run":{"completion_id":8,"result":{"Success":[],"Failure":{"code":500,"message":"x"}}}}}}),
            "result must contain exactly one variant",
        ),
        (
            "Notification: Run",
            json!({"Notification":{"Completion":{"Run":{"completion_id":8,"result":{"Failure":{"code":65536,"message":"x"}}}}}}),
            "failure code outside u16",
        ),
        (
            "Notification: CompletePromise",
            json!({"Notification":{"Completion":{"CompletePromise":{"completion_id":8,"result":{"Success":[]}}}}}),
            "result variant not allowed",
        ),
        (
            "Notification: Sleep",
            json!({"Notification":{"Completion":{"Sleep":{"completion_id":8}},"Signal":{"id":{"Index":1},"result":"Void"}}}),
            "notification must contain exactly one variant",
        ),
        (
            "Command: Sleep",
            json!({"Command":{"Sleep":{"wake_up_time":1700000120000_u64,"completion_id":7,"name":null}}}),
            "journal string malformed",
        ),
        (
            "Command: Unknown",
            json!({"Command":{"Unknown":{"name":""}}}),
            "unknown command variant",
        ),
    ] {
        let mut snapshot = observation();
        append(&mut snapshot, kind, entry)?;
        refused(&snapshot, error);
    }
    Ok(())
}

#[test]
fn guest_wake_margin_is_strict_and_pinned_identity_is_required() -> anyhow::Result<()> {
    for (wake, expected) in [
        (1700000030000_u64, None),
        (1700000030001_u64, Some(json!(7))),
    ] {
        let mut snapshot = observation();
        snapshot["journal"]["rows"][0]["entry_json"] = json!(json!({"Command":{"Sleep":{
            "wake_up_time":wake,"completion_id":7,"name":""
        }}})
        .to_string());
        assert_eq!(
            unfinished(&snapshot, &clock())?.map(|proof| proof["completion_id"].clone()),
            expected
        );
    }
    for (key, value, error) in [
        (
            "pinned_deployment_id",
            Value::Null,
            "pinned deployment absent",
        ),
        (
            "pinned_service_protocol_version",
            json!(3),
            "pinned protocol outside journal-v2 scope",
        ),
        (
            "pinned_service_protocol_version",
            json!(4294967296_u64),
            "pinned protocol outside journal-v2 scope",
        ),
    ] {
        let mut snapshot = observation();
        for side in ["before", "after"] {
            snapshot[side]["rows"][0][key] = value.clone();
        }
        refused(&snapshot, error);
    }
    Ok(())
}

#[test]
fn synthetic_combinators_are_validated_even_after_matching_completion() -> anyhow::Result<()> {
    for kind in [
        "FirstCompleted",
        "AllCompleted",
        "FirstSucceededOrAllFailed",
        "AllSucceededOrFirstFailed",
        "Unknown",
    ] {
        let mut snapshot = observation();
        future(
            &mut snapshot,
            json!({kind:[
                {"Single":{"SignalName":"interrupt"}},{"Single":{"CompletionId":7}}
            ]}),
        );
        assert_eq!(
            unfinished(&snapshot, &clock())?.map(|proof| proof["completion_id"].clone()),
            Some(json!(7)),
            "{kind}"
        );
        future(
            &mut snapshot,
            json!({kind:[
                {"Single":{"SignalName":"interrupt"}},{"Single":{"CompletionId":7,"SignalIndex":1}}
            ]}),
        );
        refused(&snapshot, "ambiguous future notification");
    }
    Ok(())
}
