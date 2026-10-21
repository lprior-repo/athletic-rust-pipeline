use super::TestResult;
use serde_json::Value;

pub(super) fn require_sleep_prefix(entries: &[Value], input: &[u8]) -> TestResult {
    let [input_entry, _, run_entry, sleep_entry] = entries else {
        return Err(
            format!("native pre-sleep journal expected four entries, got {entries:?}").into(),
        );
    };
    let kinds: Vec<_> = entries
        .iter()
        .map(|entry| entry["entry_type"].as_str())
        .collect();
    check!(eq; kinds, vec![Some("Command: Input"), Some("Command: Run"), Some("Notification: Run"), Some("Command: Sleep")]);
    for (index, entry) in entries.iter().enumerate() {
        check!(eq; entry["index"].as_u64(), Some(u64::try_from(index)?));
        check!(eq; entry["version"].as_u64(), Some(2));
        check!(entry["raw"].as_str().is_some_and(|raw| !raw.is_empty()));
        check!(entry
            .get("name")
            .is_none_or(|name| name.is_null() || name.as_str() == Some("")));
    }
    check!(eq; payload(input_entry, "/Command/Input/payload")?, input);
    check!(eq; payload(run_entry, "/Notification/Completion/Run/result/Success")?, input);
    let sleep = entry(sleep_entry)?;
    check!(eq; sleep.pointer("/Command/Sleep/completion_id").and_then(Value::as_u64), Some(2));
    Ok(())
}

pub(super) fn require_completed(entries: &[Value]) -> TestResult {
    for (kind, count) in [
        ("Command: Input", 1),
        ("Command: Run", 1),
        ("Notification: Run", 1),
        ("Command: Sleep", 1),
        ("Notification: Sleep", 1),
        ("Command: Output", 1),
    ] {
        check!(eq; entries.iter().filter(|entry| entry["entry_type"] == kind).count(), count);
    }
    check!(eq; entries.len(), 6);
    Ok(())
}

fn entry(value: &Value) -> TestResult<Value> {
    Ok(serde_json::from_str(
        value["entry_json"]
            .as_str()
            .ok_or("native entry_json absent")?,
    )?)
}

fn payload(value: &Value, pointer: &str) -> TestResult<Vec<u8>> {
    let decoded = entry(value)?;
    let payload = decoded
        .pointer(pointer)
        .ok_or("retained native payload absent")?;
    Ok(serde_json::from_value(payload.clone())?)
}
