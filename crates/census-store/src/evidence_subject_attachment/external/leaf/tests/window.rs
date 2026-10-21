use super::*;

#[test]
fn window_root_binding_is_distinct_from_reused_capture_and_preserves_original_body() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let body = b"controlled explicit full-window source";
    let capture = capture(body)?;
    let raw = serde_json::to_string(&serde_json::json!({"version":1,"window":capture}))?;
    let mut leaf = install(
        &store,
        Schema::ParticipationWindowPacketV1,
        &raw,
        RetainedCaptureSlotV1::ParticipationWindow,
        reference(body)?,
    )?;
    let before = retained_state(&store)?;
    store
        .snapshot()
        .verify_retained_capture_leaf_v1(&leaf, &mut AttachmentReplayGrantV1::new())?;
    leaf.kind = RetainedCaptureLeafKindV1::RootJson;
    leaf.object = reference(raw.as_bytes())?;
    check!(ne; leaf.object, reference(&serde_json::to_vec(&capture)?)?);
    store
        .snapshot()
        .verify_retained_capture_leaf_v1(&leaf, &mut AttachmentReplayGrantV1::new())?;
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn participation_packet_root_json_verifies_whole_packet_not_subfield() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let status_body = b"controlled status source";
    let schedule_body = b"controlled schedule source";
    let status_capture = capture(status_body)?;
    let schedule_capture = capture(schedule_body)?;
    let payload = serde_json::json!({
        "version": 1,
        "status": status_capture,
        "schedule": schedule_capture
    });
    let raw = serde_json::to_string(&payload)?;
    let mut leaf = install(
        &store,
        Schema::ParticipationPacketV1,
        &raw,
        RetainedCaptureSlotV1::ParticipationStatus,
        reference(status_body)?,
    )?;
    let before = retained_state(&store)?;
    store
        .snapshot()
        .verify_retained_capture_leaf_v1(&leaf, &mut AttachmentReplayGrantV1::new())?;
    leaf.kind = RetainedCaptureLeafKindV1::RootJson;
    leaf.object = reference(raw.as_bytes())?;
    let status_raw = serde_json::to_string(&status_capture)?;
    check!(ne; leaf.object, reference(status_raw.as_bytes())?);
    store
        .snapshot()
        .verify_retained_capture_leaf_v1(&leaf, &mut AttachmentReplayGrantV1::new())?;
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn window_root_kind_refuses_capture_substitution_and_wrong_slot() -> TestResult {
    let body = b"controlled source reused between point and window packets";
    let capture = capture(body)?;
    let window_payload = serde_json::json!({"version":1,"window":capture});
    let window_raw = serde_json::to_string(&window_payload)?;
    for case in 0..2 {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let mut leaf = install(
            &store,
            Schema::ParticipationWindowPacketV1,
            &window_raw,
            RetainedCaptureSlotV1::ParticipationWindow,
            reference(body)?,
        )?;
        leaf.kind = RetainedCaptureLeafKindV1::RootJson;
        if case == 0 {
            leaf.object = reference(&serde_json::to_vec(&capture)?)?;
        } else {
            leaf.slot = RetainedCaptureSlotV1::ParticipationStatus;
        }
        let before = retained_state(&store)?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(store
            .snapshot()
            .verify_retained_capture_leaf_v1(&leaf, &mut grant)
            .is_err());
        check!(matches!(
            grant.ensure_available(),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}

#[test]
fn participation_packet_root_kind_refuses_wrong_slot_schedule() -> TestResult {
    let body = b"controlled participation source";
    let capture = capture(body)?;
    let payload = serde_json::json!({"version":1,"status":capture,"schedule":capture});
    let raw = serde_json::to_string(&payload)?;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let mut leaf = install(
        &store,
        Schema::ParticipationPacketV1,
        &raw,
        RetainedCaptureSlotV1::ParticipationStatus,
        reference(body)?,
    )?;
    leaf.kind = RetainedCaptureLeafKindV1::RootJson;
    leaf.slot = RetainedCaptureSlotV1::ParticipationSchedule;
    let before = retained_state(&store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(store
        .snapshot()
        .verify_retained_capture_leaf_v1(&leaf, &mut grant)
        .is_err());
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn participation_packet_root_kind_refuses_capture_object_not_full_packet() -> TestResult {
    let status_body = b"controlled status body";
    let schedule_body = b"controlled schedule body";
    let status_capture = capture(status_body)?;
    let schedule_capture = capture(schedule_body)?;
    let payload = serde_json::json!({
        "version": 1,
        "status": status_capture,
        "schedule": schedule_capture
    });
    let raw = serde_json::to_string(&payload)?;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let mut leaf = install(
        &store,
        Schema::ParticipationPacketV1,
        &raw,
        RetainedCaptureSlotV1::ParticipationStatus,
        reference(status_body)?,
    )?;
    leaf.kind = RetainedCaptureLeafKindV1::RootJson;
    let status_raw = serde_json::to_string(&status_capture)?;
    leaf.object = reference(status_raw.as_bytes())?;
    let before = retained_state(&store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(store
        .snapshot()
        .verify_retained_capture_leaf_v1(&leaf, &mut grant)
        .is_err());
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn window_packet_refuses_unknown_duplicate_and_unsupported_version_before_root_acceptance(
) -> TestResult {
    let capture = serde_json::to_string(&capture(b"controlled window source")?)?;
    for raw in [
        format!(r#"{{"version":2,"window":{capture}}}"#),
        format!(r#"{{"version":1,"window":{capture},"extra":true}}"#),
        format!(r#"{{"version":1,"window":{capture},"window":{capture}}}"#),
    ] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let mut leaf = install(
            &store,
            Schema::ParticipationWindowPacketV1,
            &raw,
            RetainedCaptureSlotV1::ParticipationWindow,
            reference(raw.as_bytes())?,
        )?;
        leaf.kind = RetainedCaptureLeafKindV1::RootJson;
        let before = retained_state(&store)?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(store
            .snapshot()
            .verify_retained_capture_leaf_v1(&leaf, &mut grant)
            .is_err());
        check!(matches!(
            grant.ensure_available(),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}
