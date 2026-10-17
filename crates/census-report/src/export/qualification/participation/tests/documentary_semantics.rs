use super::*;

#[tokio::test]
async fn current_v4_archive_roundtrip_keeps_exact_descriptor_and_payload_bytes() -> TestResult {
    let fixture = Fixture::prepare(None).await?;
    let current = fixture.assess()?;
    check!(matches!(&current.replay, ParticipationReplay::V4(_)));
    let directory = fixture.objects();
    let descriptor = current.install_replay(&directory)?;
    let expected_wire = serde_json::to_vec(&current.replay)?;
    let before = super::documentary_dispatch::inventory(&directory)?;
    let loaded = ParticipationReplay::load(&directory, &descriptor)?;
    check!(matches!(&loaded, ParticipationReplay::V4(_)));
    check!(eq; serde_json::to_vec(&loaded)?, expected_wire);
    check!(eq; loaded.install(&directory)?, descriptor);
    check!(eq; super::documentary_dispatch::inventory(&directory)?, before);
    Ok(())
}

#[tokio::test]
async fn current_v4_missing_attachment_and_v3_payload_version_never_fall_back() -> TestResult {
    let fixture = Fixture::prepare(None).await?;
    let current = fixture.assess()?;
    let original = serde_json::to_value(&current.replay)?;
    let directory = fixture.objects();
    let mut missing = original.clone();
    missing.as_object_mut().ok_or("V4 object")?.remove("attachment").ok_or("attachment")?;
    let mut wrong_shape = original.clone();
    *wrong_shape.get_mut("attachment").ok_or("attachment")? = json!({});
    let mut old_version = original;
    *old_version.get_mut("version").ok_or("version")? = json!(3);
    for payload in [missing, wrong_shape] {
        let descriptor = super::documentary_dispatch::install_wire(&directory, 4, &payload)?;
        let before = super::documentary_dispatch::inventory(&directory)?;
        check!(ParticipationReplay::load(&directory, &descriptor).is_err());
        check!(eq; super::documentary_dispatch::inventory(&directory)?, before);
    }
    let descriptor = super::documentary_dispatch::install_wire(&directory, 4, &old_version)?;
    let before = super::documentary_dispatch::inventory(&directory)?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let affiliations = crate::export::qualification::current_affiliation::PinnedCurrentAffiliations::read(&dataset, &snapshot)?;
    check!(PinnedParticipation::replay(&dataset, &fixture.profile, &directory, &descriptor,
        &current.fingerprints(), affiliations.roster_context()).is_err());
    check!(eq; super::documentary_dispatch::inventory(&directory)?, before);
    Ok(())
}
