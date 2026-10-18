use super::*;
pub(super) fn read(
    snapshot: &StoreSnapshot<'_>,
    reference: &ParticipationEvidenceRefV1,
    objects: &mut Objects,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<SourceMaterial> {
    let recorded = read_recorded_independent_participation_window(snapshot, reference, grant)?
        .ok_or("window packet missing")?;
    let checked = recorded.checked();
    let document = document_only(checked.ownership(), checked.subject())?;
    let proof = checked.boundary_proof();
    let root = recorded.root().clone();
    let raw = raw_for(snapshot, &root)?;
    let slots: WindowSlots<'_> = serde_json::from_str(raw.get())?;
    let view: CaptureView<'_> = serde_json::from_str(slots.window.get())?;
    let body = body_for(&view)?;
    let body_ref = objects::reference(&body)?;
    if body_ref.digest.as_str() != view.body_sha256
        || body_ref.digest.as_str() != document.body_sha256()
        || body_ref.digest.as_str() != proof.capture()
    {
        return Err("window body digest".into());
    }
    if view.requested_url != document.document_url() {
        return Err("window url differs".into());
    }
    let quote = quote_for(proof.locator(), &body)?;
    let (capture, body_digest) = leaves(
        objects,
        &root,
        RetainedCaptureSlotV1::ParticipationWindow,
        RetainedCaptureLeafKindV1::RootJson,
        root.payload.clone(),
        body_ref,
    )?;
    let (interpretation, subject_digest, locator) = preimages(
        objects,
        document,
        proof.capture(),
        proof.locator(),
        proof.interpretation(),
        "mhsaa_subject_dcc_full_participation_window_v1",
    )?;
    let fact = objects.bytes(quote.to_vec())?;
    let context = capture.clone();
    let lineage = objects.json(&(
        "reviewed-publisher-family/v1",
        "mhsaa.com",
        document.document_url(),
        document.body_sha256(),
    ))?;
    Ok(SourceMaterial {
        subject: subject_ref(
            document,
            capture,
            body_digest,
            interpretation,
            locator,
            subject_digest,
            context,
        )?,
        fact,
        lineage,
    })
}
