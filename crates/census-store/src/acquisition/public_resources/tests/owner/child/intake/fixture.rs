use super::*;
use std::path::{Path, PathBuf};

pub(super) fn pair(
    f: &Fixture,
) -> TestResult<(
    PublicResourceCaptureOwnerV1,
    AcquisitionOccurrenceRefV1,
    AcquisitionOccurrenceRefV1,
)> {
    pair_with_roster(
        f,
        b"<html><body>controlled protocol roster</body></html>".to_vec(),
    )
}

pub(super) fn pair_with_roster(
    f: &Fixture,
    body: Vec<u8>,
) -> TestResult<(
    PublicResourceCaptureOwnerV1,
    AcquisitionOccurrenceRefV1,
    AcquisitionOccurrenceRefV1,
)> {
    let (owner, directory) = retained(f)?;
    let assessed = assessment(&f.store, &owner, &directory)?;
    let child = f
        .store
        .register_public_roster_child(f.store.prepare_public_roster_child(&assessed)?)?;
    let (capture, body) = observed::prepare_work(&f.store, child, body)?;
    observed::publish(&f.store, capture.prepared(), &body)?;
    let roster = f.store.finish_acquisition_capture(capture)?;
    Ok((owner, directory, roster))
}

pub(super) fn compare_originals(store: &Store, packet: &PublicSchoolIntakePacketV1) -> TestResult {
    for capture in [packet.directory(), packet.roster()] {
        let retained = store.read_retained_acquisition_v1(
            capture.occurrence(),
            &mut AcquisitionReadBudget::for_work(),
        )?;
        check!(eq; retained.reference(), capture.occurrence());
        check!(eq; retained.body_reference(), capture.body_reference());
        check!(eq; retained.response(), capture.response());
        check!(eq; retained.prepared().metadata_bytes(), capture.prepared().metadata_bytes());
        check!(eq; retained.prepared().content_digest(), capture.prepared().content_digest());
        check!(eq; std::fs::read(retained.body_path())?, capture.body());
        check!(capture.prepared().metadata_bytes().contains(&b'\n'));
    }
    Ok(())
}

pub(super) fn read_packet(
    store: &Store,
    reference: &PublicSchoolIntakeRefV1,
) -> TestResult<PublicSchoolIntakePacketV1> {
    let root = store
        .snapshot()
        .read_retained_capture_root_v1(
            RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1,
            reference.as_str(),
            &mut AttachmentReplayGrantV1::new(),
        )?
        .ok_or("owned intake root")?;
    Ok(PublicSchoolIntakePacketV1::decode(
        root.payload().get().as_bytes(),
    )?)
}

pub(super) fn close(f: Fixture) -> TestResult<(tempfile::TempDir, PathBuf)> {
    let Fixture {
        directory,
        store,
        writer,
        binding: _,
    } = f;
    let root = directory.path().join("store");
    store.flush()?;
    drop((store, writer));
    Ok((directory, root))
}

pub(super) fn raw_rows(root: &Path) -> TestResult<Vec<(usize, Vec<u8>, Vec<u8>)>> {
    use fjall::Readable;
    let (db, entities, journal, meta, receipts) = crate::open_keyspaces(root, 8_388_608)?;
    let snapshot = db.snapshot();
    let mut rows = Vec::new();
    for (namespace, table) in [&entities, &journal, &meta, &receipts]
        .into_iter()
        .enumerate()
    {
        for (position, guard) in snapshot.prefix(table, b"").take(513).enumerate() {
            check!(position < 512);
            let (key, value) = guard.into_inner()?;
            rows.push((namespace, key.to_vec(), value.to_vec()));
        }
    }
    Ok(rows)
}
