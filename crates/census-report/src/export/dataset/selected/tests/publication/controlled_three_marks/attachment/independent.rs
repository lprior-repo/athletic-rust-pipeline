use super::*;
fn capture_raw(
    snapshot: &StoreSnapshot<'_>,
    root: &RetainedCaptureRootV1,
    slot: &str,
) -> TestResult<Box<serde_json::value::RawValue>> {
    let raw = snapshot
        .journal_payload_raw_bounded(
            root.schema.phase(),
            &root.key,
            JournalReadBudget::new(2 * 1024 * 1024)?,
        )?
        .ok_or("retained source packet absent")?;
    let mut fields: BTreeMap<String, Box<serde_json::value::RawValue>> =
        serde_json::from_str(raw.get())?;
    Ok(fields.remove(slot).ok_or("retained source slot absent")?)
}
pub(super) fn material(
    installed: &InstalledSource,
    snapshot: &StoreSnapshot<'_>,
    participation: &RecordedIndependentParticipationV1,
    inspection: &EvidenceSubjectAttachmentInspectionV1<'_>,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<target::TargetMaterial> {
    let key = installed
        .source_records
        .iter()
        .find(|record| record.schema == RetainedCaptureRootSchemaV1::ProviderTargetPacketV1)
        .map(|record| record.key.as_str())
        .ok_or("original provider target absent")?;
    let provider =
        read_recorded_provider_target(snapshot, &SourceTargetPacketRefV1::parse(key)?, grant)?
            .ok_or("checked provider target absent")?;
    let profile = capture_raw(snapshot, provider.root(), "profile")?;
    objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: provider.root().clone(),
        slot: RetainedCaptureSlotV1::ProviderProfile,
        kind: RetainedCaptureLeafKindV1::CaptureJson,
        object: objects::reference(profile.get().as_bytes())?,
    })?;
    let status = capture_raw(snapshot, participation.root(), "status")?;
    objects.retained(RetainedCaptureLeafV1 {
        version: 1,
        root: participation.root().clone(),
        slot: RetainedCaptureSlotV1::ParticipationStatus,
        kind: RetainedCaptureLeafKindV1::CaptureJson,
        object: objects::reference(status.get().as_bytes())?,
    })?;
    #[derive(serde::Deserialize)]
    struct Body<'a> {
        body_base64: &'a str,
    }
    let wire: Body<'_> = serde_json::from_str(status.get())?;
    use base64::Engine as _;
    let body = base64::engine::general_purpose::STANDARD.decode(wire.body_base64)?;
    target::build(
        participation,
        provider.checked(),
        inspection,
        &body,
        objects,
    )
}
