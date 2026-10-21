use super::*;
use std::path::Path;

#[derive(Clone, Copy)]
enum OpenApi {
    Current,
    Identity,
}

#[derive(Clone, Copy)]
enum PublicRoots {
    Complete,
    SlotOnly,
    IdentityOnly,
}

fn current_fixture() -> TestResult<(
    tempfile::TempDir,
    Store,
    CaptureWriterV1,
    ResultsCaptureOwnerV1,
)> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("store");
    Store::create_current_qualification_v3(&root)?;
    let (store, writer) = Store::open_current_qualification_v3(&root)?.into_parts();
    store.bind_run(&RunManifest {
        store_identity: writer.store_identity().as_str().to_string(),
        run: CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("run")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![UsJurisdiction::Michigan],
    })?;
    let owner = owner(&store, &writer, 2026, CaptureCacheIntent::ReuseEligible)?;
    Ok((temporary, store, writer, owner))
}

fn public_work(store: &Store, writer: &CaptureWriterV1) -> TestResult<AcquisitionWorkV1> {
    let manifest = store.run_manifest()?.ok_or("manifest")?;
    let scope = PublicResourceCaptureScopeV1::new(
        "jurisdiction:MI:2026-27:1",
        UsJurisdiction::Michigan,
        manifest.run,
        AcquisitionDigest::parse(&serialized_digest(&manifest)?)?,
        HistoryWindow::new(2023, 2026, "2026-10-21")?,
        AcquisitionDigest::parse(&"b".repeat(64))?,
        CaptureCacheIntent::ReuseEligible,
    )?;
    Ok(AcquisitionWorkV1::for_public_resource(
        writer.store_identity().clone(),
        &scope,
        UsJurisdiction::Michigan,
        PublicResourceObligationV1::new(
            PublicSourceResourceV1::milesplit_athlete_profile("13749840")?,
            ProfileDate::parse("2026-10-21")?,
        ),
        RequestRepresentationV1::normalized_get(
            "https://mi.milesplit.com/athletes/13749840-brennan-smith",
            vec![
                ("accept".into(), "*/*".into()),
                ("accept-encoding".into(), "identity".into()),
                ("user-agent".into(), "Census/1.0".into()),
            ],
            "Census/1.0",
        )?,
    )?)
}

fn inject(store: &Store, work: &AcquisitionWorkV1, roots: PublicRoots) -> TestResult {
    if matches!(roots, PublicRoots::Complete | PublicRoots::SlotOnly) {
        overwrite(
            store,
            &format!("slot/{}/binding", work.slot().id()?),
            &facts::SlotBinding {
                version: CaptureProtocolVersion::V2,
                slot: work.slot().clone(),
                work_id: work.id()?,
            },
        )?;
    }
    if matches!(roots, PublicRoots::Complete | PublicRoots::IdentityOnly) {
        overwrite(store, &format!("work/{}/identity", work.id()?), work)?;
    }
    Ok(())
}

fn cold_open(root: &Path, api: OpenApi) -> StoreResult<Store> {
    match api {
        OpenApi::Current => {
            let (store, writer) = Store::open_current_qualification_v3(root)?.into_parts();
            drop(writer);
            Ok(store)
        }
        OpenApi::Identity => {
            let (store, writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
            drop(writer);
            Ok(store)
        }
    }
}

fn cold_rows(root: &Path) -> TestResult<Vec<(Vec<u8>, Vec<u8>)>> {
    check!(root.join(crate::DB_DIR).join("version").is_file());
    let (db, _entities, journal, meta, receipts) = crate::open_keyspaces(root, 8_388_608)?;
    let mut rows = Vec::new();
    for keyspace in [&meta, &journal, &receipts] {
        for (position, guard) in db.snapshot().prefix(keyspace, b"").take(1025).enumerate() {
            check!(position < 1024);
            let (key, value) = guard.into_inner()?;
            rows.push((key.to_vec(), value.to_vec()));
        }
    }
    Ok(rows)
}

#[test]
fn complete_public_roots_without_enrollment_refuse_both_cold_opens_without_mutation() -> TestResult
{
    let (temporary, store, writer, owner) = current_fixture()?;
    let work = public_work(&store, &writer)?;
    let original = rows(&store)?;
    inject(&store, &work, PublicRoots::Complete)?;
    let before = rows(&store)?;
    check!(eq; before.len(), original.len() + 2);
    let root = temporary.path().join("store");
    drop((owner, writer, store));
    for api in [OpenApi::Current, OpenApi::Identity] {
        check!(matches!(
            cold_open(&root, api),
            Err(StoreError::Capture(
                CaptureError::CaptureCapabilityNamespaceUnsupported
            ))
        ));
        check!(eq; cold_rows(&root)?, before);
    }
    Ok(())
}

#[test]
fn orphan_public_slot_or_identity_refuses_both_cold_opens_without_mutation() -> TestResult {
    for roots in [PublicRoots::SlotOnly, PublicRoots::IdentityOnly] {
        let (temporary, store, writer, owner) = current_fixture()?;
        let work = public_work(&store, &writer)?;
        let original = rows(&store)?;
        inject(&store, &work, roots)?;
        let before = rows(&store)?;
        check!(eq; before.len(), original.len() + 1);
        let root = temporary.path().join("store");
        drop((owner, writer, store));
        for api in [OpenApi::Current, OpenApi::Identity] {
            check!(matches!(
                cold_open(&root, api),
                Err(StoreError::Capture(CaptureError::SlotWorkMismatch))
            ));
            check!(eq; cold_rows(&root)?, before);
        }
    }
    Ok(())
}

#[test]
fn legacy_registration_survives_both_cold_opens_with_exact_identity_work_and_rows() -> TestResult {
    let (temporary, store, writer, owner) = current_fixture()?;
    let owned = registered(&store, &writer, &owner)?;
    let work = owned.work().clone();
    let id = owned.work_id().clone();
    let identity = store.store_identity()?;
    let before = rows(&store)?;
    store.flush()?;
    let root = temporary.path().join("store");
    drop((owned, owner, writer, store));
    for api in [OpenApi::Current, OpenApi::Identity] {
        let store = cold_open(&root, api)?;
        check!(eq; store.store_identity()?, identity);
        check!(eq; rows(&store)?, before);
        let facts = read::read_work(
            &store.db.snapshot(),
            &store.journal,
            &store.receipts,
            &id,
            &mut AcquisitionReadBudget::for_work(),
            CaptureProtocolVersion::V2,
        )?;
        check!(eq; facts.work, work);
        check!(eq; facts.work.id()?, id);
        check!(facts.attempts.is_empty());
        check!(facts.result.is_none());
        check!(matches!(facts.decision(), AcquireDecision::Ready));
        check!(eq; rows(&store)?, before);
        drop(store);
        check!(eq; cold_rows(&root)?, before);
    }
    Ok(())
}
