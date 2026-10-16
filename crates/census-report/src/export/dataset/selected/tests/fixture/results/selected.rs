use super::{append_meet, synthetic_body, CanonicalPerformance, Store, TestResult, RAW};
use serde_json::{json, Value};

pub(in super::super) async fn append_baseline(store: &Store) -> TestResult<CanonicalPerformance> {
    let body = body("725218", "1266814", 201782263, "F", "13.00")?;
    append_meet(
        store,
        "https://al.milesplit.com/meets/725218/results/1266814/raw",
        &body,
        RAW,
        "milesplit_result:201782263",
    )
    .await
}

pub(in super::super) async fn append_outside_scope(store: &Store) -> TestResult {
    for (meet, result_set, result, gender, mark, sport) in [
        (
            "725219",
            "1266815",
            201782264,
            "F",
            "12.00",
            "Indoor Track & Field",
        ),
        (
            "725220",
            "1266816",
            201782265,
            "M",
            "11.00",
            "Outdoor Track & Field",
        ),
        (
            "725221",
            "1266817",
            201782266,
            "M",
            "10.00",
            "Indoor Track & Field",
        ),
    ] {
        let body = body(meet, result_set, result, gender, mark)?;
        let raw = std::str::from_utf8(RAW)?
            .replace("725218", meet)
            .replace("1266814", result_set)
            .replace("Outdoor Track & Field", sport)
            .replace(
                "Troy Invitational #2",
                &format!("Synthetic Scope Meet {meet}"),
            )
            .replace(
                "Girls Results",
                if gender == "M" {
                    "Boys Results"
                } else {
                    "Girls Results"
                },
            );
        append_meet(
            store,
            &format!("https://al.milesplit.com/meets/{meet}/results/{result_set}/raw"),
            &body,
            raw.as_bytes(),
            &format!("milesplit_result:{result}"),
        )
        .await?;
    }
    Ok(())
}

fn body(
    meet: &str,
    result_set: &str,
    result: u64,
    gender: &str,
    mark: &str,
) -> TestResult<Vec<u8>> {
    let mut body: Value = serde_json::from_slice(&synthetic_body("High School")?)?;
    *body.pointer_mut("/_embedded/meet/id").ok_or("meet owner")? = json!(meet);
    let row = body
        .pointer_mut("/data/0")
        .and_then(Value::as_object_mut)
        .ok_or("result row")?;
    for (key, value) in [
        ("meetId", meet),
        ("meetResultsId", result_set),
        ("gender", gender),
        ("eventName", "100 Meter Dash"),
        ("eventCode", "100m"),
        ("mark", mark),
    ] {
        row.insert(key.into(), json!(value));
    }
    row.insert("id".into(), json!(result));
    row.insert("eventDistance".into(), json!(100));
    row.remove("units");
    if gender == "M" {
        row.insert("athleteId".into(), json!("14222593"));
        row.insert("firstName".into(), json!("Synthetic"));
        row.insert("lastName".into(), json!("Boy"));
        row.insert(
            "profileUrl".into(),
            json!("https://www.milesplit.com/athletes/14222593-synthetic-boy"),
        );
    }
    Ok(serde_json::to_vec(&body)?)
}
