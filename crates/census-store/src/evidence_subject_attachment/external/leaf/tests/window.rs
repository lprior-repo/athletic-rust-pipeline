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
fn window_root_kind_refuses_capture_substitution_wrong_slot_and_other_root_schema() -> TestResult {
    let body = b"controlled source reused between point and window packets";
    let capture = capture(body)?;
    for case in 0..3 {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let (schema, slot, payload) = if case == 2 {
            (
                Schema::ParticipationPacketV1,
                RetainedCaptureSlotV1::ParticipationStatus,
                serde_json::json!({"version":1,"status":capture,"schedule":capture}),
            )
        } else {
            (
                Schema::ParticipationWindowPacketV1,
                RetainedCaptureSlotV1::ParticipationWindow,
                serde_json::json!({"version":1,"window":capture}),
            )
        };
        let raw = serde_json::to_string(&payload)?;
        let mut leaf = install(&store, schema, &raw, slot, reference(raw.as_bytes())?)?;
        leaf.kind = RetainedCaptureLeafKindV1::RootJson;
        if case == 0 {
            leaf.object = reference(&serde_json::to_vec(&capture)?)?;
        } else if case == 1 {
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
