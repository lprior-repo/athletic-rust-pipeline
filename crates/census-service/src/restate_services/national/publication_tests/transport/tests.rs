use super::*;

#[test]
fn captured_protocol_flags_survive_exact_wire_roundtrip() -> TestResult {
    let bytes = [0x04, 0x0d, 0x80, 0x01, 0, 0, 0, 2, 0x08, 0x01];
    let frames = codec::frames(&bytes)?;
    check!(eq; frames.first().ok_or("frame absent")?.encode()?, bytes);
    Ok(())
}

#[test]
fn protocol_varint_overflow_is_refused_instead_of_truncated() -> TestResult {
    let frame = codec::Frame::new(0, vec![8, 255, 255, 255, 255, 255, 255, 255, 255, 255, 2]);
    check!(frame.number(1).is_err());
    Ok(())
}

#[test]
fn truncated_frame_headers_and_payloads_are_refused() -> TestResult {
    for bytes in [vec![0; 7], vec![4, 13, 0, 0, 0, 0, 0, 2, 8]] {
        check!(codec::frames(&bytes).is_err());
    }
    Ok(())
}

#[test]
fn frame_count_limit_is_exact_and_refuses_the_next_frame() -> TestResult {
    let mut bytes = vec![0; codec::MAX_FRAMES * 8];
    check!(eq; codec::frames(&bytes)?.len(), codec::MAX_FRAMES);
    bytes.extend([0; 8]);
    check!(codec::frames(&bytes).is_err());
    Ok(())
}

#[test]
fn terminal_framing_refuses_empty_mixed_duplicate_and_out_of_order_endings() -> TestResult {
    let output = codec::Frame::new(0x0401, Vec::new());
    let end = codec::Frame::new(0x0003, Vec::new());
    let suspend = codec::Frame::new(0x0001, Vec::new());
    check!(eq; terminal(&[output.clone(), end.clone()])?, 0x0003);
    for frames in [
        Vec::new(),
        vec![end.clone()],
        vec![end.clone(), output.clone()],
        vec![output.clone(), end.clone(), suspend.clone()],
        vec![suspend.clone(), suspend],
        vec![output.clone(), output, end],
    ] {
        check!(terminal(&frames).is_err());
    }
    Ok(())
}

#[test]
fn response_and_status_capacity_is_reserved_before_request_dispatch() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut transcript = Transcript::new(root.path().to_path_buf());
    transcript.bytes =
        MAX_TRANSCRIPT_BYTES - codec::MAX_WIRE_BYTES - MAX_STATUS_BYTES - PENDING.len();
    check!(transcript.begin(b"x").is_err());
    check!(eq; std::fs::read_dir(root.path())?.count(), 0);
    transcript.bytes -= 1;
    transcript.begin(b"x")?;
    check!(eq; std::fs::read(root.path().join("old-0000.request.bin"))?, b"x");
    Ok(())
}

#[test]
fn interrupted_exchange_retains_an_explicit_pending_marker() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut transcript = Transcript::new(root.path().to_path_buf());
    transcript.begin(b"request before cancellation")?;
    check!(eq; std::fs::read(root.path().join("old-0000.pending.json"))?, PENDING);
    check!(!root.path().join("old-0000.status.json").exists());
    Ok(())
}

#[test]
fn a_failed_transport_keeps_its_exact_request_and_failure_status() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut transcript = Transcript::new(root.path().to_path_buf());
    let stem = transcript.begin(b"retained request")?;
    transcript.finish(&stem, None, b"", Some("connection refused".to_owned()))?;
    let status: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.path().join("old-0000.status.json"))?)?;
    check!(eq; status, serde_json::json!({"status": null, "error": "connection refused"}));
    check!(eq; std::fs::read(root.path().join("old-0000.request.bin"))?, b"retained request");
    Ok(())
}

#[test]
fn wire_admission_accepts_exact_limit_and_refuses_one_more_before_append() -> TestResult {
    let mut bytes = vec![0; codec::MAX_WIRE_BYTES - 1];
    append(&mut bytes, &[1])?;
    check!(eq; bytes.len(), codec::MAX_WIRE_BYTES);
    check!(append(&mut bytes, &[2]).is_err());
    check!(eq; bytes.last(), Some(&1));
    check!(codec::frames(&vec![0; codec::MAX_WIRE_BYTES + 1]).is_err());
    Ok(())
}

#[test]
fn transcript_keeps_exact_exchange_bytes_and_never_overwrites() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut transcript = Transcript::new(root.path().to_path_buf());
    transcript.record(b"request", 200, b"response")?;
    check!(eq; std::fs::read(root.path().join("old-0000.request.bin"))?, b"request");
    check!(eq; std::fs::read(root.path().join("old-0000.response.bin"))?, b"response");
    let mut duplicate = Transcript::new(root.path().to_path_buf());
    check!(duplicate.record(b"changed", 200, b"changed").is_err());
    check!(eq; std::fs::read(root.path().join("old-0000.request.bin"))?, b"request");
    Ok(())
}

#[test]
fn transcript_budget_refusal_precedes_any_file_creation() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut transcript = Transcript::new(root.path().to_path_buf());
    transcript.bytes = MAX_TRANSCRIPT_BYTES;
    check!(transcript.record(b"x", 200, b"y").is_err());
    check!(eq; std::fs::read_dir(root.path())?.count(), 0);
    Ok(())
}
