use super::*;

#[test]
fn school_root_owns_both_capture_slots_with_existing_combined_cost() -> TestResult {
    let body = b"controlled original school capture";
    let capture = capture(body)?;
    let packet = serde_json::json!({"version":1,"interpretation":"milesplit_directory_roster_school_v1","directory":capture,"roster":capture});
    let raw = serde_json::to_string(&packet)?;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    check!(eq; Schema::ProviderSchoolPacketV1.phase(), "provider_school_intake_v1");
    for slot in [
        RetainedCaptureSlotV1::ProviderSchoolDirectory,
        RetainedCaptureSlotV1::ProviderSchoolRoster,
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
        check!(eq; grant.usage().traversal_steps(), 64);
        check!(eq; grant.usage().encoded_bytes(), 0);
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
        check!(eq; owned_grant.usage().traversal_steps(), 50);
        check!(eq; owned_grant.usage().encoded_bytes(), raw.len());
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
