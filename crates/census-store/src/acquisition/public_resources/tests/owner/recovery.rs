use super::*;

mod maximal;
mod padded;
use crate::acquisition::{
    ExistingRecoveryRoute, PreparedBackingAssessment, PreparedRecoveryBudget,
};
use crate::acquisition::{PreparedRecoveryRead, RetainedCaptureParts};

#[test]
fn observed_prepared_capture_cold_recovery_preserves_exact_heads_body_metadata_and_receipt(
) -> TestResult {
    let f = Fixture::new()?;
    f.declare("2026-10-14T04:00:00Z")?;
    let owner = f
        .writer
        .public_resource_owner(&f.store, UsJurisdiction::Michigan)?;
    let work = owner.for_root(&f.store, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let (prepared, body) = observed::prepare(&f.store, &f.writer, &owner)?;
    let attempt = prepared.attempt_id().clone();
    let input = prepared.prepared().clone();
    observed::publish(&f.store, &input, &body)?;
    let prepared_key = Store::journal_key(
        CaptureProtocolVersion::V2.phase(),
        &format!(
            "work/{}/attempt/{}/prepared",
            work.id()?,
            attempt.ordinal().get()
        ),
    );
    let prepared_bytes = f
        .store
        .journal
        .get(&prepared_key)?
        .ok_or("prepared journal bytes")?
        .to_vec();
    let before = Witness::capture(&f.store)?;
    let root = f.directory.path().join("store");
    let Fixture {
        directory,
        store,
        writer,
        binding: _,
    } = f;
    store.flush()?;
    drop((prepared, owner, writer, store));
    for current in [true, false] {
        let store = cold_open(&root, current)?;
        check!(eq; Witness::capture(&store)?, before);
        drop(store);
    }
    let (store, writer) = Store::open_current_qualification_v3(&root)?.into_parts();
    let owner = writer.public_resource_owner(&store, UsJurisdiction::Michigan)?;
    check!(eq; owner.for_root(&store, PublicResourceRootKindV1::MileSplitTeamDirectory)?, work);
    let owned = registered(&store, &writer, &owner, work.clone())?;
    check!(eq; store.inspect_acquisition_work(&owned, &mut AcquisitionReadBudget::for_work())?, crate::AcquireDecision::FinishPrepared(attempt.clone()));
    let mut selection = AcquisitionReadBudget::for_selection();
    let ExistingRecoveryRoute::Prepared(target) =
        store.read_existing_recovery_target(&owner, &work, &mut selection)?
    else {
        return Err("public prepared recovery target absent".into());
    };
    let usage = selection.selection_usage();
    let mut budget = PreparedRecoveryBudget::admit(usage, 1)?;
    store.validate_prepared_recovery_owner(&owner, &mut budget)?;
    let PreparedRecoveryRead::Unresolved(session) = store
        .begin_prepared_recovery(&owner, &target, &mut budget)?
        .read_backing()?
    else {
        return Err("public prepared recovery is not unresolved".into());
    };
    check!(matches!(
        session.assessment()?,
        PreparedBackingAssessment::Exact
    ));
    let parts = session.finish()?.into_parts();
    check!(eq; &parts.body, &body);
    check!(eq; &parts.prepared, &input);
    check!(eq; parts.occurrence.attempt(), &attempt);
    check!(eq; parts.original_occurrence, parts.occurrence);
    check!(eq; parts.fetched_at, "2026-10-14T05:00:01Z");
    check!(eq; store.journal.get(&prepared_key)?.ok_or("held prepared journal")?.to_vec(), prepared_bytes);
    replay_exact(&store, &owner, &target, usage, &parts)?;
    let retained = store
        .read_retained_acquisition_v1(&parts.occurrence, &mut AcquisitionReadBudget::for_work())?;
    check!(eq; std::fs::read(retained.body_path())?, body);
    check!(eq; retained.prepared().metadata_bytes(), input.metadata_bytes());
    drop((owner, writer, store, directory));
    Ok(())
}

fn replay_exact(
    store: &Store,
    owner: &PublicResourceCaptureOwnerV1,
    target: &PreparedRecoveryTarget,
    usage: crate::AcquisitionSelectionUsage,
    original: &RetainedCaptureParts,
) -> TestResult {
    let before = Witness::capture(store)?;
    let mut budget = PreparedRecoveryBudget::admit(usage, 1)?;
    let PreparedRecoveryRead::Retained(retained) = store
        .begin_prepared_recovery(owner, target, &mut budget)?
        .read_backing()?
    else {
        return Err("public prepared recovery replay did not retain original".into());
    };
    let repeated = retained.into_parts();
    check!(eq; repeated.occurrence, original.occurrence);
    check!(eq; repeated.receipt, original.receipt);
    check!(eq; repeated.body, original.body);
    check!(eq; repeated.prepared, original.prepared);
    check!(eq; Witness::capture(store)?, before);
    let facts = crate::acquisition::read::read_work(
        &store.db.snapshot(),
        &store.journal,
        &store.receipts,
        target.attempt().work_id(),
        &mut AcquisitionReadBudget::for_work(),
        CaptureProtocolVersion::V2,
    )?;
    check!(eq; facts.attempts.len(), 1);
    check!(eq; facts.decision(), crate::AcquireDecision::Replay(original.occurrence.clone()));
    Ok(())
}

#[test]
fn cached_owner_admission_still_requires_exact_frontier_entry_for_prepared_work() -> TestResult {
    let f = Fixture::new()?;
    f.declare("2026-10-14T04:00:00Z")?;
    let owner = f
        .writer
        .public_resource_owner(&f.store, UsJurisdiction::Michigan)?;
    let work = owner.for_root(&f.store, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let (prepared, body) = observed::prepare(&f.store, &f.writer, &owner)?;
    observed::publish(&f.store, prepared.prepared(), &body)?;
    let mut selection = AcquisitionReadBudget::for_selection();
    let ExistingRecoveryRoute::Prepared(target) =
        f.store
            .read_existing_recovery_target(&owner, &work, &mut selection)?
    else {
        return Err("public prepared recovery target absent".into());
    };
    let mut budget = PreparedRecoveryBudget::admit(selection.selection_usage(), 1)?;
    f.store
        .validate_prepared_recovery_owner(&owner, &mut budget)?;
    let id =
        crate::acquisition::public_resources::frontier::entry_id(&declaration(&f.store)?, &work)?;
    f.store
        .journal
        .remove(format!("{ENTRY_PREFIX}{id}").as_bytes())?;
    f.store.flush()?;
    let before = Witness::capture(&f.store)?;
    check!(
        matches!(f.store.begin_prepared_recovery(&owner, &target, &mut budget),
        Err(StoreError::Refused { detail }) if detail == "public resource frontier entry absent")
    );
    check!(eq; Witness::capture(&f.store)?, before);
    Ok(())
}
