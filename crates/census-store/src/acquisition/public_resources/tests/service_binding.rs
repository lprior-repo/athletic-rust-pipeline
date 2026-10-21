use super::super::{codec, PublicResourceDeclarationBindingV1};
use super::fixture::*;
use crate::{CaptureError, PublicResourceCaptureOwnerV1, Store, StoreError, StoreResult};
use census_domain::model::ProfileSelectionV1;
use census_domain::UsJurisdiction;

fn require(
    owner: &PublicResourceCaptureOwnerV1,
    store: &Store,
    binding: &PublicResourceDeclarationBindingV1,
) -> StoreResult<()> {
    let jurisdiction = binding.owner(owner.scope().jurisdiction())?;
    owner.require_service_binding(
        store,
        &binding.selection,
        &binding.national_owner_key,
        &binding.national_request_digest,
        &jurisdiction.jurisdiction_request_digest,
        &jurisdiction.source_plan_fingerprint,
    )
}

#[test]
fn exact_service_binding_checks_are_read_only_and_replay_without_registration() -> TestResult {
    let f = Fixture::new()?;
    let receipt = f.declare("2026-10-14T04:00:00Z")?.receipt().clone();
    let owner = f
        .writer
        .public_resource_owner(&f.store, UsJurisdiction::Michigan)?;
    let before = Witness::capture(&f.store)?;
    for _ in 0..2 {
        require(&owner, &f.store, &f.binding)?;
        check!(eq; Witness::capture(&f.store)?, before);
        check!(eq; f.store.receipt(&receipt.operation)?, Some(receipt.clone()));
    }
    Ok(())
}

#[test]
fn every_service_binding_input_substitution_refuses_without_writes() -> TestResult {
    let f = Fixture::new()?;
    f.declare("2026-10-14T04:00:00Z")?;
    let owner = f
        .writer
        .public_resource_owner(&f.store, UsJurisdiction::Michigan)?;
    let mut cases = Vec::new();
    let mut changed = f.binding.clone();
    changed.selection =
        ProfileSelectionV1::new(changed.store.clone(), profile("2026-10-15")?.id()?);
    cases.push(changed);
    let mut changed = f.binding.clone();
    changed.national_owner_key.push_str(":other");
    cases.push(changed);
    let mut changed = f.binding.clone();
    changed.national_request_digest = different_digest(&changed.national_request_digest)?;
    cases.push(changed);
    let mut changed = f.binding.clone();
    let jurisdiction = changed.owners.first_mut().ok_or("jurisdiction binding")?;
    jurisdiction.jurisdiction_request_digest =
        different_digest(&jurisdiction.jurisdiction_request_digest)?;
    cases.push(changed);
    let mut changed = f.binding.clone();
    let jurisdiction = changed.owners.first_mut().ok_or("jurisdiction binding")?;
    jurisdiction.source_plan_fingerprint = different_digest(&jurisdiction.source_plan_fingerprint)?;
    cases.push(changed);
    let before = Witness::capture(&f.store)?;
    for changed in cases {
        refused(
            require(&owner, &f.store, &changed),
            "public resource service binding",
        )?;
        check!(eq; Witness::capture(&f.store)?, before);
    }
    Ok(())
}

#[test]
fn foreign_store_refuses_even_with_the_original_exact_service_inputs() -> TestResult {
    let f = Fixture::new()?;
    f.declare("2026-10-14T04:00:00Z")?;
    let owner = f
        .writer
        .public_resource_owner(&f.store, UsJurisdiction::Michigan)?;
    let other = Fixture::new()?;
    other.declare("2026-10-14T04:00:00Z")?;
    let before = Witness::capture(&f.store)?;
    let foreign_before = Witness::capture(&other.store)?;
    check!(matches!(
        require(&owner, &other.store, &f.binding),
        Err(StoreError::Capture(
            CaptureError::CaptureCapabilityForeignStore
        ))
    ));
    check!(eq; Witness::capture(&f.store)?, before);
    check!(eq; Witness::capture(&other.store)?, foreign_before);
    Ok(())
}

#[test]
fn service_binding_revalidates_the_current_protected_receipt_after_owner_creation() -> TestResult {
    let f = Fixture::new()?;
    f.declare("2026-10-14T04:00:00Z")?;
    let owner = f
        .writer
        .public_resource_owner(&f.store, UsJurisdiction::Michigan)?;
    let mut receipt = declaration(&f.store)?.receipt()?;
    receipt.at = timestamp("2026-10-15T04:00:00Z")?.to_rfc3339();
    f.store
        .receipts
        .insert(receipt.operation.as_bytes(), codec::encode(&receipt)?)?;
    let before = Witness::capture(&f.store)?;
    refused(
        require(&owner, &f.store, &f.binding),
        "public resource declaration receipt",
    )?;
    check!(eq; Witness::capture(&f.store)?, before);
    Ok(())
}

#[test]
fn service_binding_revalidates_the_current_profile_after_owner_creation() -> TestResult {
    let f = Fixture::new()?;
    f.declare("2026-10-14T04:00:00Z")?;
    let owner = f
        .writer
        .public_resource_owner(&f.store, UsJurisdiction::Michigan)?;
    let key = crate::profiles::key(f.binding.selection.profile_id());
    f.store.meta.remove(key.as_bytes())?;
    let before = Witness::capture(&f.store)?;
    refused(
        require(&owner, &f.store, &f.binding),
        "public resource selected profile absent",
    )?;
    check!(eq; Witness::capture(&f.store)?, before);
    Ok(())
}

#[test]
fn current_cold_owners_and_identity_cold_view_preserve_original_service_binding() -> TestResult {
    let f = Fixture::new()?;
    f.declare("2026-10-14T04:00:00Z")?;
    let before = Witness::capture(&f.store)?;
    let Fixture {
        directory,
        store,
        writer,
        binding,
    } = f;
    let root = directory.path().join("store");
    store.flush()?;
    drop((store, writer));
    for _ in 0..2 {
        let (store, writer) = Store::open_current_qualification_v3(&root)?.into_parts();
        let owner = writer.public_resource_owner(&store, UsJurisdiction::Michigan)?;
        require(&owner, &store, &binding)?;
        require(&owner, &store, &binding)?;
        check!(eq; Witness::capture(&store)?, before);
        drop((owner, writer, store));
        let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        check!(eq; Witness::capture(&store)?, before);
        drop((writer, store));
    }
    drop(directory);
    Ok(())
}
