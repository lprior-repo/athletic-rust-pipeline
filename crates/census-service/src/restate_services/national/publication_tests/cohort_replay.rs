use super::{codec, journal, TestResult};
use crate::restate_services::wire::NationalRequest;
use std::sync::Arc;

fn original_prefix() -> TestResult<journal::Journal> {
    let metadata: serde_json::Value =
        serde_json::from_str(include_str!("original_d6_bind_prefix.json"))?;
    let mut frames = Vec::new();
    for entry in metadata["frames"].as_array().ok_or("original frames")? {
        let kind = match entry["entry_type"].as_str().ok_or("original entry type")? {
            "Command: Input" => 0x0400,
            "Command: Call" => 0x040D,
            "Command: SetState" => 0x0403,
            "Notification: CallInvocationId" => 0x800E,
            "Notification: Call" => 0x800D,
            other => return Err(format!("unsupported original entry {other}").into()),
        };
        let hex = entry["raw_hex"].as_str().ok_or("original protobuf")?;
        let mut pairs = hex.as_bytes().chunks_exact(2);
        let payload = pairs
            .by_ref()
            .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
            .collect::<TestResult<Vec<_>>>()?;
        if !pairs.remainder().is_empty() {
            return Err("truncated original hexadecimal byte".into());
        }
        frames.push(codec::Frame::new(kind, payload));
    }
    let input = frames.first().ok_or("original Input")?;
    let request: NationalRequest =
        serde_json::from_slice(codec::field(input.bytes(14)?, 1)?.ok_or("original Input value")?)?;
    let mut replay = journal::Journal::new(&request)?;
    replay.entries = frames;
    replay.invocation = metadata["invocation"]
        .as_str()
        .ok_or("original invocation")?
        .to_owned();
    replay.identity = metadata["workflow_key"]
        .as_str()
        .ok_or("original workflow key")?
        .to_owned();
    Ok(replay)
}

#[tokio::test]
async fn original_d6_bind_prefix_refuses_legacy_contract_without_dispatch() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Arc::new(census_store::Store::open(directory.path())?);
    let error = original_prefix()
        .err()
        .ok_or("legacy d6 bind prefix decoded")?;
    check!(
        error.to_string().contains("selected_publication_v1"),
        "{error}"
    );
    check!(store.run_manifest()?.is_none());
    Ok(())
}
