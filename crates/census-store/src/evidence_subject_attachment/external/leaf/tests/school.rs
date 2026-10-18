use super::*;

#[test]
fn school_root_owns_both_capture_slots_for_consumer_verification() -> TestResult {
    let directory_body: &[u8] = b"controlled original school directory";
    let roster_body: &[u8] = b"controlled original school roster";
    let directory_capture = capture(directory_body)?;
    let roster_capture = capture(roster_body)?;
    let packet = serde_json::json!({"version":1,"interpretation":"milesplit_directory_roster_school_v1","directory":directory_capture,"roster":roster_capture});
    let raw = serde_json::to_string(&packet)?;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    check!(eq; Schema::ProviderSchoolPacketV1.phase(), "provider_school_intake_v1");
    for (slot, body, other_body) in [
        (
            RetainedCaptureSlotV1::ProviderSchoolDirectory,
            directory_body,
            roster_body,
        ),
        (
            RetainedCaptureSlotV1::ProviderSchoolRoster,
            roster_body,
            directory_body,
        ),
    ] {
        let leaf = install(
            &store,
            Schema::ProviderSchoolPacketV1,
            &raw,
            slot,
            reference(body)?,
        )?;
        let before = retained_state(&store)?;
        let mut grant = AttachmentReplayGrantV1::new();
        store
            .snapshot()
            .verify_retained_capture_leaf_v1(&leaf, &mut grant)?;
        let mut swapped = leaf.clone();
        swapped.object = reference(other_body)?;
        let mut refused_grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            store
                .snapshot()
                .verify_retained_capture_leaf_v1(&swapped, &mut refused_grant),
            Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(_)))
        ));
        check!(matches!(
            refused_grant.ensure_available(),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
        check!(eq; retained_state(&store)?, before);
        let mut owned_grant = AttachmentReplayGrantV1::new();
        let held = store
            .snapshot()
            .read_retained_capture_root_v1(
                Schema::ProviderSchoolPacketV1,
                &leaf.root.key,
                &mut owned_grant,
            )?
            .ok_or("missing school root")?;
        check!(eq; held.root(), &leaf.root);
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}

#[test]
fn school_root_refuses_wrong_interpretation_duplicate_map_shape_and_bad_other_capture() -> TestResult
{
    let capture = serde_json::to_string(&capture(b"context")?)?;
    let bad = capture.replacen("{", "{\"unknown\":true,", 1);
    for raw in [
        format!(
            r#"{{"version":2,"interpretation":"milesplit_directory_roster_school_v1","directory":{capture},"roster":{capture}}}"#
        ),
        format!(
            r#"{{"version":1,"interpretation":"milesplit_profile_linked_competition_target_v1","directory":{capture},"roster":{capture}}}"#
        ),
        format!(
            r#"{{"version":1,"interpretation":"milesplit_directory_roster_school_v1","directory":{capture},"roster":{capture},"\u0072oster":{capture}}}"#
        ),
        format!(
            r#"{{"version":1,"interpretation":"milesplit_directory_roster_school_v1","directory":{capture},"roster":{bad}}}"#
        ),
        format!(r#"[1,"milesplit_directory_roster_school_v1",{capture},{capture}]"#),
    ] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let leaf = install(
            &store,
            Schema::ProviderSchoolPacketV1,
            &raw,
            RetainedCaptureSlotV1::ProviderSchoolDirectory,
            reference(b"context")?,
        )?;
        let before = retained_state(&store)?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            store.snapshot().read_retained_capture_root_v1(
                Schema::ProviderSchoolPacketV1,
                &leaf.root.key,
                &mut grant
            ),
            Err(StoreError::Decode { .. }
                | StoreError::EvidenceSubjectAttachment(Error::Corrupt(_)))
        ));
        check!(matches!(
            grant.ensure_available(),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}
