use super::*;
use census_store::replay_result_school_source_proof_v1;

pub(super) struct Proof {
    bytes: Vec<u8>,
    reference: AttachmentObjectReferenceV1,
    store: StoreIdentity,
    sequence: u64,
}

pub(super) fn current(store: &Store) -> crate::CrawlResult<Vec<ResultSchoolReviewedRecordV1>> {
    current_with_grant(store, &mut AttachmentReplayGrantV1::new())
}

pub(super) fn current_with_grant(
    store: &Store,
    grant: &mut AttachmentReplayGrantV1,
) -> crate::CrawlResult<Vec<ResultSchoolReviewedRecordV1>> {
    let snapshot = store.snapshot();
    let attachments = snapshot.read_current_evidence_subject_attachment_references_v1(grant)?;
    let schools = snapshot.read_current_result_school_correspondences_v1(&attachments, grant)?;
    let discovered = discover_current_html_results(&attachments, grant)?;
    if discovered.group_count() != 1 || discovered.diagnostic_count() != 0 {
        return Err(missing("current exact capture group"));
    }
    discovered.with_group(0, grant, |linked, grant| {
        source_rows(&linked)?;
        (0..2)
            .map(|index| {
                let checked = read::checked_current(&schools, &attachments, &linked, index, grant)?
                    .ok_or_else(|| missing("accepted current school"))?;
                Ok(checked.record().clone())
            })
            .collect()
    })
}

fn source_rows(
    linked: &DiscoveredIdentityLinkedHtmlResultDocumentV1<'_, '_>,
) -> crate::CrawlResult<()> {
    if linked.row_count() != 2 {
        return Err(missing("exact two accepted source rows"));
    }
    let first = linked.row(0).ok_or_else(|| missing("first original row"))?;
    let second = linked
        .row(1)
        .ok_or_else(|| missing("second original row"))?;
    if first.checked_row().context_reference() == second.checked_row().context_reference()
        || first.checked_row().context_reference() == linked.recorded_root().key
        || second.checked_row().context_reference() == linked.recorded_root().key
    {
        return Err(missing("row-specific original contexts"));
    }
    for (index, meet, day) in [(0, "95201", "2026-03-21"), (1, "93917", "2026-03-11")] {
        let row = linked
            .row(index)
            .ok_or_else(|| missing("original row index"))?;
        let CheckedHtmlResultRowV1::Tfrrs(checked) = row.checked_row() else {
            return Err(missing("original source namespace"));
        };
        if checked.meet_id() != meet || checked.published_date() != day {
            return Err(missing("per-row native meet and exact day"));
        }
    }
    Ok(())
}

pub(super) fn verify(
    records: &[ResultSchoolReviewedRecordV1],
    input: &fixture::Input,
) -> TestResult {
    check!(eq; records.len(), 2);
    for (index, record) in records.iter().enumerate() {
        let scope = scope_for(index)?;
        let expected = ["Columbus North", "Hamilton Southeastern"]
            .get(index)
            .copied()
            .ok_or("verified index")?;
        check!(eq; record.command.source.statement.statement_text.as_str(), expected);
        check!(eq; record.command.source.target.day, scope.day);
        check!(eq; record.command.source.target.school, input.school);
        check!(record
            .command
            .rationale
            .as_str()
            .contains("controlled cross-provider"));
    }
    Ok(())
}

pub(super) fn freeze(store: &Store) -> TestResult<Proof> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let current =
        snapshot.read_current_result_school_correspondences_v1(&attachments, &mut grant)?;
    let identity = snapshot.freeze_public_identity_v2()?;
    let proof = current.freeze_closure(&identity, &mut grant)?;
    Ok(Proof {
        bytes: proof.bytes().to_vec(),
        reference: proof.reference().clone(),
        store: current.store_identity().clone(),
        sequence: current.snapshot_sequence(),
    })
}

pub(super) fn frozen(
    proof: &Proof,
    input: &fixture::Input,
) -> crate::CrawlResult<Vec<ResultSchoolReviewedRecordV1>> {
    frozen_with_grant(proof, input, &mut AttachmentReplayGrantV1::new())
}

pub(super) fn frozen_with_grant(
    proof: &Proof,
    input: &fixture::Input,
    grant: &mut AttachmentReplayGrantV1,
) -> crate::CrawlResult<Vec<ResultSchoolReviewedRecordV1>> {
    let current = replay_result_school_source_proof_v1(
        &proof.bytes,
        &proof.reference,
        &proof.store,
        proof.sequence,
        grant,
    )?;
    let root = current
        .read_retained_capture_root_v1(
            RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
            &input.context,
            grant,
        )?
        .ok_or_else(|| missing("frozen original TFRRS root"))?;
    let capture: RetainedHtmlCaptureV1 =
        serde_json::from_str(root.payload().get()).map_err(crate::participation_evidence::json)?;
    if capture.body()?.as_bytes() != GENUINE_BODY {
        return Err(missing("frozen original source bytes"));
    }
    let attachments = current.attachments(grant)?;
    let discovered = discover_frozen_html_results(attachments, grant)?;
    if discovered.group_count() != 1 || discovered.diagnostic_count() != 0 {
        return Err(missing("frozen exact capture group"));
    }
    discovered.with_group(0, grant, |linked, grant| {
        source_rows(&linked)?;
        (0..2)
            .map(|index| {
                let checked = read::checked_frozen(&current, &linked, index, grant)?
                    .ok_or_else(|| missing("accepted frozen school"))?;
                Ok(checked.record().clone())
            })
            .collect()
    })
}
