use super::*;
use census_domain::model::{EventKind, ExactSeconds, Mark, TimingMethod};

#[test]
fn genuine_html_capture_reopens_exact_document_rows_without_person_attachment() -> TestResult {
    check!(eq; hash(HTML), "95350f1389f9f81fc0354d73637a59cf508413f8bea530a91eb0de7c28cc19e5");
    let (directory, store) = fresh_store()?;
    let before = protected_state(&store)?;
    let tables = store.snapshot().tables_digest(&Table::ALL)?;
    let capture = html_capture()?;
    let requested = locators();
    let original = read_retained_html_result_rows(&capture, &requested)?;
    let reference = original.context_reference().to_string();
    let payload = serde_json::to_value(&capture)?;
    let recording = Recording::new();
    record_retained_html_result_capture(&store, &recording, &original)?;
    check!(eq; protected_state(&store)?, before);
    let recorded = recording.drain();
    check!(eq; recorded, expected_journal(RetainedCaptureRootSchemaV1::HtmlResultCaptureV1.phase(),
        &reference, payload.clone()));
    let prepared = recorded.prepare("recorded-html-reopen")?;
    let first = prepared.apply(&store)?;
    check!(eq; first.appended(), 0);
    retained_only_tables(&store, &tables)?;
    no_public_birth(&store)?;
    drop(store);
    let store = reopen(&directory)?;
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let reopened =
        read_recorded_html_result_capture(&snapshot, &reference, &requested, &mut grant)?
            .ok_or("HTML capture holder")?;
    exact_root(
        &snapshot,
        reopened.root(),
        RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
        RetainedCaptureRootSchemaV1::HtmlResultCaptureV1.phase(),
        &reference,
        &payload,
    )?;
    check!(eq; serde_json::to_value(reopened.capture())?, payload);
    exact_capture(&payload, None, HTML)?;
    let document = reopened.read(&requested)?;
    check!(eq; document.original_document().as_bytes(), HTML);
    check!(eq; document.context_reference(), original.context_reference());
    check!(eq; document.parsed_page(), original.parsed_page());
    check!(eq; document.meet_id(), "763888");
    check!(eq; document.result_document_id(), "1304896");
    check!(eq; document.selected_count(), 3);
    for (index, (mark, kind)) in [
        ("13.57", EventKind::Track110mHurdles),
        ("14.10", EventKind::Track110mHurdles),
        ("36.23", EventKind::Track300mHurdles),
    ]
    .into_iter()
    .enumerate()
    {
        let row = document.row(index).ok_or("reopened original row")?;
        let before = original.row(index).ok_or("original selected row")?;
        check!(eq; row.locator(), *requested.get(index).ok_or("requested locator")?);
        check!(eq; row.original_row()?.as_bytes(), before.original_row()?.as_bytes());
        check!(eq; row.original_row_digest(), before.original_row_digest());
        check!(eq; row.reference(), before.reference());
        check!(eq; row.subject()?, before.subject()?);
        check!(eq; row.parsed_row().mark, Mark::TimeSeconds(ExactSeconds::parse(mark)?));
        check!(eq; row.parsed_row().timing, Some(TimingMethod::Fat));
        check!(eq; row.event().kind, kind);
    }
    let raw = snapshot
        .journal_payload_raw_bounded(
            RetainedCaptureRootSchemaV1::HtmlResultCaptureV1.phase(),
            &reference,
            JournalReadBudget::new(2 * 1024 * 1024)?,
        )?
        .ok_or("independent installed raw HTML")?;
    let owned_bytes = raw.get().len();
    check!(eq; grant.usage().encoded_bytes(), owned_bytes);
    let first_usage = grant.usage();
    let expected_root = reopened.root().clone();
    drop(reopened);
    check!(eq; grant.usage(), first_usage);
    let reread = read_recorded_html_result_capture(&snapshot, &reference, &requested, &mut grant)?
        .ok_or("same-grant original HTML")?;
    check!(eq; reread.root(), &expected_root);
    check!(eq; grant.usage().encoded_bytes(), owned_bytes.checked_mul(2).ok_or("owned-byte overflow")?);
    check!(grant.usage().traversal_steps() > first_usage.traversal_steps());
    let installed = protected_state(&store)?;
    let repeated = prepared.apply(&store)?;
    check!(eq; repeated.receipt(), first.receipt());
    check!(eq; repeated.appended(), 0);
    check!(eq; protected_state(&store)?, installed);
    retained_only_tables(&store, &tables)?;
    no_public_birth(&store)?;
    Ok(())
}

#[test]
fn one_tampered_requested_html_locator_refuses_without_changing_retained_state() -> TestResult {
    let (directory, store) = fresh_store()?;
    let capture = html_capture()?;
    let original = read_retained_html_result_rows(&capture, &locators())?;
    let reference = original.context_reference().to_string();
    let recording = Recording::new();
    record_retained_html_result_capture(&store, &recording, &original)?;
    let retained = install_for_refusal(recording.drain(), &store, "html-request-refusal")?;
    drop(store);
    let store = reopen(&directory)?;
    let before = protected_state(&store)?;
    let mut changed = locators();
    changed.last_mut().ok_or("last locator")?.byte_length = 91;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(
        matches!(read_recorded_html_result_capture(&store.snapshot(), &reference, &changed, &mut grant),
        Err(CrawlError::Store(StoreError::Refused { detail }))
        if detail == "requested row is not an exact successful original row")
    );
    check!(eq; protected_state(&store)?, before);
    require_retained_state(&store, &retained)?;
    let snapshot = store.snapshot();
    check!(matches!(
        read_recorded_html_result_capture(&snapshot, &reference, &locators(), &mut grant),
        Err(CrawlError::Store(StoreError::EvidenceSubjectAttachment(
            census_store::EvidenceSubjectAttachmentStoreError::Domain(
                EvidenceSubjectAttachmentError::Poisoned
            )
        )))
    ));
    require_retained_state(&store, &retained)?;
    check!(eq; protected_state(&store)?, before);
    let mut independent_read = AttachmentReplayGrantV1::new();
    let held = read_recorded_html_result_capture(
        &snapshot,
        &reference,
        &locators(),
        &mut independent_read,
    )?
    .ok_or("unmodified original still readable")?;
    check!(eq; held.read(&locators())?.selected_count(), 3);
    no_public_birth(&store)?;
    Ok(())
}
