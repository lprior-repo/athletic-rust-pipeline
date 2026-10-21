use super::*;
use crate::participation_evidence::{RetainedBodyEncodingV1, RetainedHttpProvenanceV1};
use census_domain::public_identity_history::RegistryHead;
use census_store::PublicIdentityInputRevisionsV2;
use chrono::{DateTime, Utc};

pub(super) const BODY: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../research/sources/national-aggregators/samples/directathletics-tfrrs/indiana-tfrrs-list-5489.html"));
pub(super) const SCHEMA: RetainedCaptureRootSchemaV1 =
    RetainedCaptureRootSchemaV1::HtmlResultCaptureV1;
pub(super) fn locators() -> [SourceRowLocator; 2] {
    [
        SourceRowLocator {
            ordinal: 118,
            byte_offset: 205297,
            byte_length: 1409,
        },
        SourceRowLocator {
            ordinal: 119,
            byte_offset: 206710,
            byte_length: 1360,
        },
    ]
}

pub(super) fn genuine_body_with_controlled_transport() -> TestResult<RetainedHtmlCaptureV1> {
    let url = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
    Ok(RetainedHtmlCaptureV1::new(
        url,
        url,
        200,
        DateTime::parse_from_rfc3339("2026-09-22T03:55:31Z")?.with_timezone(&Utc),
        BODY,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![("Accept".into(), "text/html".into())],
            response_header_bytes: b"HTTP/2 200\r\ncontent-type: text/html; charset=utf-8\r\n\r\n"
                .to_vec(),
            content_type: "text/html; charset=utf-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: DateTime::parse_from_rfc3339("2026-09-22T03:55:30Z")?.with_timezone(&Utc),
        },
    )?)
}

pub(super) fn fresh() -> TestResult<(tempfile::TempDir, Store)> {
    let directory = tempfile::tempdir()?;
    Store::create_public_identity_qualification_v3(directory.path().join("store"))?;
    let store = reopen(&directory)?;
    Ok((directory, store))
}

pub(super) fn reopen(directory: &tempfile::TempDir) -> TestResult<Store> {
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("store"))?.into_parts();
    drop(writer);
    Ok(store)
}

pub(super) fn recorded(store: &Store) -> TestResult<(census_store::recorded::Recorded, String)> {
    let capture = genuine_body_with_controlled_transport()?;
    let list = read_retained_tfrrs_list_rows(&capture, &locators())?;
    let recording = Recording::new();
    record_retained_tfrrs_list_capture(store, &recording, &list)?;
    Ok((recording.drain(), list.context_reference().into()))
}

pub(super) fn install(
    store: &Store,
    recorded: census_store::recorded::Recorded,
    operation: &str,
) -> TestResult {
    crate::recording::prepare_recorded(recorded, operation)?.apply(store)?;
    Ok(())
}

type ProtectedState = (
    u64,
    u64,
    u64,
    u64,
    String,
    PublicIdentityInputRevisionsV2,
    RegistryHead,
);

pub(super) fn state(store: &Store) -> TestResult<ProtectedState> {
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("qualification identity ledger")?;
    check!(eq; accepted.head().sequence, 0);
    check!(eq; accepted.public_ids().count(), 0);
    check!(eq; accepted.history_records().len(), 0);
    Ok((
        snapshot.sequence(),
        snapshot.evidence_generation(),
        snapshot.derived_generation(),
        store.receipt_count()?,
        snapshot.tables_digest(&Table::ALL)?,
        snapshot.public_identity_input_revisions_v2()?,
        accepted.head().clone(),
    ))
}

pub(super) fn raw(store: &Store, key: &str) -> TestResult<String> {
    Ok(store
        .snapshot()
        .journal_payload_raw_bounded(
            SCHEMA.phase(),
            key,
            JournalReadBudget::new(4 * 1024 * 1024)?,
        )?
        .ok_or("retained TFRRS root")?
        .get()
        .into())
}

pub(super) fn present(
    store: &Store,
    key: &str,
    requested: &[SourceRowLocator],
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<bool> {
    read_recorded_tfrrs_list_capture(&store.snapshot(), key, requested, grant)
        .map(|held| held.is_some())
}

pub(super) fn poisoned(result: CrawlResult<bool>) -> TestResult {
    check!(matches!(result, Err(crate::CrawlError::Store(census_store::StoreError::EvidenceSubjectAttachment(
        census_store::EvidenceSubjectAttachmentStoreError::Domain(
            census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError::Poisoned))))));
    Ok(())
}
