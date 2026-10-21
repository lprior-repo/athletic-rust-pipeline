use super::*;
use census_store::{JournalReadBudget, Table};

pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
pub(super) type State = (
    u64,
    u64,
    u64,
    String,
    census_domain::public_identity_history::RegistryHead,
);

pub(super) fn fresh() -> TestResult<(tempfile::TempDir, Store)> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    drop(writer);
    Ok((directory, store))
}

pub(super) fn reopen(directory: &tempfile::TempDir) -> TestResult<Store> {
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("store"))?.into_parts();
    drop(writer);
    Ok(store)
}

pub(super) fn state(store: &Store) -> TestResult<State> {
    let snapshot = store.snapshot();
    Ok((
        snapshot.evidence_generation(),
        snapshot.derived_generation(),
        store.receipt_count()?,
        snapshot.tables_digest(&Table::ALL)?,
        snapshot
            .accepted_public_identity_snapshot()?
            .ok_or("current identity ledger")?
            .head()
            .clone(),
    ))
}

pub(super) fn unchanged_authority(store: &Store, before: &State) -> TestResult {
    let after = state(store)?;
    check!(eq; after.0, before.0);
    check!(eq; after.1, before.1);
    check!(eq; after.3, before.3);
    check!(eq; after.4, before.4);
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("current identity ledger")?;
    check!(eq; accepted.head().sequence, 0);
    check!(eq; accepted.history_records().len(), 0);
    check!(eq; accepted.public_ids().count(), 0);
    Ok(())
}

pub(super) fn stage(
    store: &Store,
    packet: &MhsaaCoopPdfPacketV1,
) -> TestResult<crate::recording::PreparedRecorded> {
    let recording = Recording::new();
    let before = state(store)?;
    let reference = record_mhsaa_coop_pdf_packet(store, &recording, packet)?;
    check!(eq; reference, packet.reference()?);
    check!(eq; state(store)?, before);
    let recorded = recording.drain();
    check!(recorded.rows.is_empty());
    check!(recorded.effects.is_empty());
    check!(eq; recorded.journal.len(), 1);
    check!(eq; recorded.journal[0].phase, MHSAA_COOP_PDF_CAPTURE_PHASE);
    check!(eq; recorded.journal[0].key, reference.as_str());
    Ok(crate::recording::prepare_recorded(
        recorded,
        "mhsaa-coop-pdf-genuine",
    )?)
}

pub(super) fn exact(
    store: &Store,
    packet: &MhsaaCoopPdfPacketV1,
) -> TestResult<census_store::RetainedCaptureLeafV1> {
    let snapshot = store.snapshot();
    let reference = packet.reference()?;
    let mut grant = AttachmentReplayGrantV1::new();
    let held = read_recorded_mhsaa_coop_pdf_packet(&snapshot, &reference, &mut grant)?
        .ok_or("retained PDF packet")?;
    check!(eq; held.packet().encode()?, packet.encode()?);
    check!(eq; held.packet().capture().original_bytes(), packet.capture().original_bytes());
    check!(eq; held.root().schema, RetainedCaptureRootSchemaV1::MhsaaCoopPdfPacketV1);
    check!(eq; held.root().key, reference.as_str());
    let raw = snapshot
        .journal_payload_raw_bounded(
            MHSAA_COOP_PDF_CAPTURE_PHASE,
            reference.as_str(),
            JournalReadBudget::new(MAX_MHSAA_COOP_PDF_PACKET_BYTES)?,
        )?
        .ok_or("original packet payload")?;
    check!(eq; raw.get().as_bytes(), packet.encode()?);
    check!(eq; held.root().payload.digest.as_str(), super::super::super::hash(raw.get().as_bytes()));
    check!(eq; held.root().payload.bytes, u64::try_from(raw.get().len())?);
    let checked = held.read(&mut grant)?;
    check!(eq; checked.candidate().primary().name(), "Pickford");
    check!(eq; checked.candidate().program_row().page(), 24);
    let leaf = census_store::RetainedCaptureLeafV1 {
        version: 1,
        root: held.root().clone(),
        slot: census_store::RetainedCaptureSlotV1::SelfCapture,
        kind: census_store::RetainedCaptureLeafKindV1::OriginalBody,
        object: packet.original_body_reference()?,
    };
    snapshot.verify_retained_capture_leaf_v1(&leaf, &mut grant)?;
    let mut root_json = leaf.clone();
    root_json.kind = census_store::RetainedCaptureLeafKindV1::RootJson;
    root_json.object = held.root().payload.clone();
    snapshot.verify_retained_capture_leaf_v1(&root_json, &mut grant)?;
    Ok(leaf)
}
