use super::super::{codec, read, PublicResourceFrontierEntryV1, PublicResourceRootKindV1};
use super::super::{ENTRY_PREFIX, FRONTIER_OPERATION_PREFIX};
use super::fixture::*;
use crate::acquisition::{facts::SlotBinding, AcquisitionReadBudget, CaptureError};
use crate::{
    CaptureWriterV1, OwnedAcquisitionWork, PublicResourceCaptureOwnerV1, Store, StoreError,
};
use census_domain::model::*;
use census_domain::UsJurisdiction;

mod child;
mod observed;
mod recovery;

fn registered(
    store: &Store,
    writer: &CaptureWriterV1,
    owner: &PublicResourceCaptureOwnerV1,
    work: AcquisitionWorkV1,
) -> TestResult<OwnedAcquisitionWork> {
    let grant = writer.continuation_for_public_resource(store, owner, &work)?;
    Ok(store.register_acquisition_work(owner, work, grant)?)
}

#[test]
fn registration_commits_frontier_slot_work_and_receipt_once_and_replays_exactly() -> TestResult {
    let f = Fixture::new()?;
    f.declare("2026-10-14T04:00:00Z")?;
    let owner = f
        .writer
        .public_resource_owner(&f.store, UsJurisdiction::Michigan)?;
    let work = owner.for_root(&f.store, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let before = Witness::capture(&f.store)?;
    let owned = registered(&f.store, &f.writer, &owner, work.clone())?;
    let after = Witness::capture(&f.store)?;
    check!(eq; after.sequence, before.sequence.checked_add(1).ok_or("registration sequence")?);
    check!(eq; after.journal.len(), before.journal.len().checked_add(3).ok_or("journal rows")?);
    check!(eq; after.receipts.len(), before.receipts.len().checked_add(1).ok_or("receipt rows")?);
    check!(eq; after.meta, before.meta);
    check!(eq; after.entities, before.entities);
    verify_registration(&f.store, &work)?;
    check!(eq; f.store.inspect_acquisition_work(&owned, &mut AcquisitionReadBudget::for_work())?, crate::AcquireDecision::Ready);
    let repeated = registered(&f.store, &f.writer, &owner, work.clone())?;
    check!(eq; repeated.work_id(), owned.work_id());
    check!(eq; repeated.work(), &work);
    check!(eq; Witness::capture(&f.store)?, after);
    Ok(())
}

fn verify_registration(store: &Store, work: &AcquisitionWorkV1) -> TestResult {
    let declaration = declaration(store)?;
    let id = super::super::frontier::entry_id(&declaration, work)?;
    let snapshot = store.db.snapshot();
    let entry = read::entry(
        &snapshot,
        &store.journal,
        &store.receipts,
        format!("{ENTRY_PREFIX}{id}").as_bytes(),
        &declaration,
        &mut AcquisitionReadBudget::for_work(),
    )?;
    check!(eq; &entry.work, work);
    let operation = format!("{FRONTIER_OPERATION_PREFIX}{id}");
    let receipt = store
        .receipt(&operation)?
        .ok_or("frontier registration receipt")?;
    super::super::format::validate_entry_receipt(&declaration, &entry, &receipt)?;
    let slot: SlotBinding = crate::acquisition::codec::get_fact(
        &snapshot,
        &store.journal,
        &format!("slot/{}/binding", work.slot().id()?),
        &mut AcquisitionReadBudget::for_work(),
        CaptureProtocolVersion::V2,
    )?
    .ok_or("registered slot binding")?;
    check!(eq; slot.slot, *work.slot());
    check!(eq; slot.work_id, work.id()?);
    let facts = crate::acquisition::read::read_work(
        &snapshot,
        &store.journal,
        &store.receipts,
        &work.id()?,
        &mut AcquisitionReadBudget::for_work(),
        CaptureProtocolVersion::V2,
    )?;
    check!(eq; facts.work, *work);
    check!(facts.attempts.is_empty());
    Ok(())
}

#[test]
fn orphan_frontier_refusal_cannot_commit_staged_slot_or_work() -> TestResult {
    let f = Fixture::new()?;
    f.declare("2026-10-14T04:00:00Z")?;
    let owner = f
        .writer
        .public_resource_owner(&f.store, UsJurisdiction::Michigan)?;
    let work = owner.for_root(&f.store, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let declaration = declaration(&f.store)?;
    let entry = PublicResourceFrontierEntryV1::new(
        &declaration,
        UsJurisdiction::Michigan,
        PublicResourceRootKindV1::MileSplitTeamDirectory,
        work.request().clone(),
    )?;
    f.store.journal.insert(
        format!("{ENTRY_PREFIX}{}", entry.id()?).as_bytes(),
        codec::encode(&entry)?,
    )?;
    f.store.flush()?;
    let grant = f
        .writer
        .continuation_for_public_resource(&f.store, &owner, &work)?;
    let before = Witness::capture(&f.store)?;
    check!(
        matches!(f.store.register_acquisition_work(&owner, work.clone(), grant),
        Err(StoreError::Refused { detail }) if detail == "public resource frontier already present without registration")
    );
    check!(eq; Witness::capture(&f.store)?, before);
    for key in [
        format!("slot/{}/binding", work.slot().id()?),
        format!("work/{}/identity", work.id()?),
    ] {
        check!(!f
            .store
            .journal
            .contains_key(Store::journal_key(CaptureProtocolVersion::V2.phase(), &key))?);
    }
    Ok(())
}

#[test]
fn directory_robots_and_other_site_grants_cannot_substitute_slots_or_owners() -> TestResult {
    let f = Fixture::for_sites(vec![UsJurisdiction::Michigan, UsJurisdiction::Colorado])?;
    f.declare("2026-10-14T04:00:00Z")?;
    let mi = f
        .writer
        .public_resource_owner(&f.store, UsJurisdiction::Michigan)?;
    let co = f
        .writer
        .public_resource_owner(&f.store, UsJurisdiction::Colorado)?;
    let directory = mi.for_root(&f.store, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let robots = mi.for_root(&f.store, PublicResourceRootKindV1::Robots)?;
    let colorado = co.for_root(&f.store, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    check!(ne; directory.slot().id()?, robots.slot().id()?);
    check!(ne; directory.slot().id()?, colorado.slot().id()?);
    let grant = f
        .writer
        .continuation_for_public_resource(&f.store, &mi, &robots)?;
    let before = Witness::capture(&f.store)?;
    check!(matches!(
        f.store
            .register_acquisition_work(&mi, directory.clone(), grant),
        Err(StoreError::Capture(CaptureError::SlotWorkMismatch))
    ));
    let grant = f
        .writer
        .continuation_for_public_resource(&f.store, &mi, &directory)?;
    check!(matches!(
        f.store.register_acquisition_work(&co, colorado, grant),
        Err(StoreError::Capture(CaptureError::SlotWorkMismatch))
    ));
    check!(eq; Witness::capture(&f.store)?, before);
    registered(&f.store, &f.writer, &mi, directory)?;
    registered(&f.store, &f.writer, &mi, robots)?;
    Ok(())
}

#[test]
fn undeclared_owner_arbitrary_resource_role_and_foreign_store_refuse_without_writes() -> TestResult
{
    let f = Fixture::new()?;
    let before = Witness::capture(&f.store)?;
    check!(matches!(
        f.writer
            .public_resource_owner(&f.store, UsJurisdiction::Michigan),
        Err(StoreError::Capture(
            CaptureError::CaptureCapabilityNamespaceUnsupported
        ))
    ));
    check!(eq; Witness::capture(&f.store)?, before);
    f.declare("2026-10-14T04:00:00Z")?;
    let owner = f
        .writer
        .public_resource_owner(&f.store, UsJurisdiction::Michigan)?;
    let root = owner.for_root(&f.store, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let arbitrary = AcquisitionWorkV1::for_public_resource(
        f.writer.store_identity().clone(),
        owner.scope(),
        UsJurisdiction::Michigan,
        PublicResourceObligationV1::new(
            PublicSourceResourceV1::milesplit_athlete_profile("12345")?,
            ProfileDate::parse("2026-10-14")?,
        ),
        RequestRepresentationV1::normalized_get(
            "https://mi.milesplit.com/athletes/12345-controlled",
            root.request().representation_headers().to_vec(),
            root.request().effective_user_agent(),
        )?,
    )?;
    let grant = f
        .writer
        .continuation_for_public_resource(&f.store, &owner, &root)?;
    let before = Witness::capture(&f.store)?;
    check!(
        matches!(f.store.register_acquisition_work(&owner, arbitrary, grant),
        Err(StoreError::Refused { detail }) if detail == "public resource frontier root role")
    );
    let foreign = Fixture::new()?;
    foreign.declare("2026-10-14T04:00:00Z")?;
    let other = foreign
        .writer
        .public_resource_owner(&foreign.store, UsJurisdiction::Michigan)?;
    let grant = f
        .writer
        .continuation_for_public_resource(&f.store, &owner, &root)?;
    check!(matches!(
        f.store.register_acquisition_work(&other, root, grant),
        Err(StoreError::Capture(
            CaptureError::CaptureCapabilityForeignStore
        ))
    ));
    check!(eq; Witness::capture(&f.store)?, before);
    Ok(())
}

#[test]
fn both_cold_qualification_opens_preserve_complete_public_roots_and_canonical_ids() -> TestResult {
    let f = Fixture::new()?;
    f.declare("2026-10-14T04:00:00Z")?;
    let owner = f
        .writer
        .public_resource_owner(&f.store, UsJurisdiction::Michigan)?;
    let directory_work =
        owner.for_root(&f.store, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let robots_work = owner.for_root(&f.store, PublicResourceRootKindV1::Robots)?;
    registered(&f.store, &f.writer, &owner, directory_work.clone())?;
    registered(&f.store, &f.writer, &owner, robots_work.clone())?;
    let original = Witness::capture(&f.store)?;
    let identity = f.writer.store_identity().clone();
    let root = f.directory.path().join("store");
    let Fixture {
        directory,
        store,
        writer,
        binding: _,
    } = f;
    store.flush()?;
    drop((owner, writer, store));
    for current in [true, false] {
        let store = cold_open(&root, current)?;
        check!(eq; store.store_identity()?, identity);
        check!(eq; Witness::capture(&store)?, original);
        for work in [&directory_work, &robots_work] {
            verify_registration(&store, work)?;
        }
        check!(eq; Witness::capture(&store)?, original);
        drop(store);
    }
    drop(directory);
    Ok(())
}

fn cold_open(root: &std::path::Path, current: bool) -> TestResult<Store> {
    if current {
        let (store, writer) = Store::open_current_qualification_v3(root)?.into_parts();
        drop(writer);
        Ok(store)
    } else {
        let (store, writer) = Store::open_public_identity_qualification_v3(root)?.into_parts();
        drop(writer);
        Ok(store)
    }
}
