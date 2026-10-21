use super::super::{
    codec, PublicResourceDeclarationV1, DECLARATION, ENTRY_PREFIX, FRONTIER_OPERATION_PREFIX,
    OPERATION_PREFIX,
};
use super::fixture::*;
use crate::{Application, CaptureError, ProfileRegistryError, ReplayFrontier, Store, StoreError};
use census_domain::model::{
    AcquisitionDigest, CensusRun, ProfileSelectionV1, QueryProfileV1, SchoolYear, StoreIdentity,
};
use census_domain::UsJurisdiction;
use fjall::Readable;

const FIRST: &str = "2026-10-14T05:00:00Z";
const LATER: &str = "2026-10-15T06:00:00Z";

#[test]
fn declaration_replay_with_changed_clock_preserves_original_receipt_and_every_row() -> TestResult {
    let fixture = Fixture::new()?;
    let before = Witness::capture(&fixture.store)?;
    let application = fixture.declare(FIRST)?;
    check!(matches!(application, Application::Written(_)));
    let receipt = application.receipt().clone();
    check!(eq; receipt.appended, 0);
    check!(!receipt.credited);
    check!(eq; receipt.at, timestamp(FIRST)?.to_rfc3339());
    let retained = declaration(&fixture.store)?;
    check!(eq; retained.binding, fixture.binding);
    check!(eq; retained.admitted_at, timestamp(FIRST)?);
    check!(eq; retained.receipt()?, receipt);
    let declared = Witness::capture(&fixture.store)?;
    check!(eq; declared.sequence, before.sequence.checked_add(1).ok_or("sequence")?);
    check!(eq; declared.journal.len(), before.journal.len() + 1);
    check!(eq; declared.receipts.len(), before.receipts.len() + 1);
    check!(eq; declared.entities, before.entities);
    check!(eq; declared.meta, before.meta);
    for clock in ["2026-10-13T04:00:00Z", LATER] {
        check!(eq; fixture.declare(clock)?, Application::Repeated(receipt.clone()));
        check!(eq; Witness::capture(&fixture.store)?, declared);
        check!(eq; declaration(&fixture.store)?, retained);
    }
    Ok(())
}

#[test]
fn changed_binding_refuses_replay_without_replacing_the_declaration() -> TestResult {
    let fixture = Fixture::new()?;
    fixture.declare(FIRST)?;
    let before = Witness::capture(&fixture.store)?;
    let mut changed = fixture.binding.clone();
    let owner = changed.owners.first_mut().ok_or("owner")?;
    owner.source_plan_fingerprint = different_digest(&owner.source_plan_fingerprint)?;
    check!(matches!(
        fixture.store.declare_public_resource_capture(&fixture.writer, changed, timestamp(LATER)?),
        Err(StoreError::Refused { detail }) if detail == "public resource declaration binding conflict"
    ));
    check!(eq; Witness::capture(&fixture.store)?, before);
    check!(eq; declaration(&fixture.store)?.binding, fixture.binding);
    Ok(())
}

#[test]
fn foreign_lineage_and_selected_profile_inputs_refuse_without_any_store_write() -> TestResult {
    let fixture = Fixture::new()?;
    let alternate = profile("2026-10-15")?;
    fixture.store.register_profile(&alternate)?;
    let mut input = profile("2026-10-14")?.input().clone();
    input.jurisdictions = vec![UsJurisdiction::Colorado];
    let outside_scope = QueryProfileV1::new(input)?;
    fixture.store.register_profile(&outside_scope)?;
    let identity = AcquisitionDigest::parse(fixture.writer.store_identity().as_str())?;
    let foreign = StoreIdentity::parse(&different_digest(&identity)?.to_string())?;
    check!(ne; &foreign, fixture.writer.store_identity());
    let mut candidates = Vec::new();
    let mut changed = fixture.binding.clone();
    changed.store = foreign.clone();
    candidates.push(changed);
    let mut changed = fixture.binding.clone();
    changed.run_manifest_digest = different_digest(&changed.run_manifest_digest)?;
    candidates.push(changed);
    let mut changed = fixture.binding.clone();
    changed.capture_declaration_digest = different_digest(&changed.capture_declaration_digest)?;
    candidates.push(changed);
    let mut changed = fixture.binding.clone();
    changed.run = CensusRun::new(SchoolYear::DEFAULT, 2).ok_or("foreign run")?;
    candidates.push(changed);
    let mut changed = fixture.binding.clone();
    changed.selection = ProfileSelectionV1::new(foreign, changed.selection.profile_id().clone());
    candidates.push(changed);
    let mut changed = fixture.binding.clone();
    changed.selection = ProfileSelectionV1::new(changed.store.clone(), alternate.id()?);
    candidates.push(changed);
    let mut changed = fixture.binding.clone();
    changed.selection = ProfileSelectionV1::new(changed.store.clone(), outside_scope.id()?);
    candidates.push(changed);
    let before = Witness::capture(&fixture.store)?;
    for candidate in candidates {
        check!(fixture
            .store
            .declare_public_resource_capture(&fixture.writer, candidate, timestamp(FIRST)?,)
            .is_err());
        check!(eq; Witness::capture(&fixture.store)?, before);
    }
    check!(fixture.declare(FIRST)?.written());
    Ok(())
}

#[test]
fn absent_selected_profile_refuses_without_registering_a_profile_or_declaration() -> TestResult {
    let fixture = Fixture::new()?;
    let missing = profile("2026-10-16")?.id()?;
    let mut binding = fixture.binding.clone();
    binding.selection = ProfileSelectionV1::new(binding.store.clone(), missing.clone());
    let before = Witness::capture(&fixture.store)?;
    check!(matches!(
        fixture.store.declare_public_resource_capture(&fixture.writer, binding, timestamp(FIRST)?),
        Err(StoreError::ProfileRegistry(ProfileRegistryError::MissingProfile { id })) if id == missing
    ));
    check!(eq; Witness::capture(&fixture.store)?, before);
    Ok(())
}

#[test]
fn writer_from_a_different_qualified_store_cannot_declare_or_mutate_either_store() -> TestResult {
    let fixture = Fixture::new()?;
    let foreign = Fixture::new()?;
    let before = Witness::capture(&fixture.store)?;
    let other_before = Witness::capture(&foreign.store)?;
    check!(matches!(
        fixture.store.declare_public_resource_capture(
            &foreign.writer,
            fixture.binding.clone(),
            timestamp(FIRST)?,
        ),
        Err(StoreError::Capture(
            CaptureError::CaptureCapabilityForeignStore
        ))
    ));
    check!(eq; Witness::capture(&fixture.store)?, before);
    check!(eq; Witness::capture(&foreign.store)?, other_before);
    Ok(())
}

#[test]
fn declaration_without_frontier_entries_survives_both_current_and_identity_cold_opens() -> TestResult
{
    let fixture = Fixture::new()?;
    let receipt = fixture.declare(FIRST)?.receipt().clone();
    let expected = declaration(&fixture.store)?;
    let held = Witness::capture(&fixture.store)?;
    let selection = fixture.binding.selection.clone();
    check!(fixture
        .store
        .db
        .snapshot()
        .prefix(&fixture.store.journal, ENTRY_PREFIX)
        .next()
        .is_none());
    let Fixture {
        directory,
        store,
        writer,
        binding,
    } = fixture;
    let root = directory.path().join("store");
    store.flush()?;
    drop((store, writer));
    let (store, writer) = Store::open_current_qualification_v3(&root)?.into_parts();
    check!(eq; declaration(&store)?, expected);
    check!(eq; store.store_identity()?, binding.store);
    check!(eq; store.selected_profile(&selection)?.id()?, *selection.profile_id());
    check!(eq; Witness::capture(&store)?, held);
    check!(eq; store.declare_public_resource_capture(&writer, binding, timestamp(LATER)?)?, Application::Repeated(receipt.clone()));
    drop((store, writer));
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    check!(eq; declaration(&store)?, expected);
    check!(eq; store.store_identity()?, selection.store_identity().clone());
    check!(eq; store.receipt(&receipt.operation)?, Some(receipt));
    check!(eq; Witness::capture(&store)?, held);
    drop((store, writer));
    Ok(())
}

#[test]
fn forged_reserved_roots_fail_both_cold_open_paths_with_or_without_a_declaration() -> TestResult {
    for declared in [false, true] {
        for damage in [
            "meta-capture",
            "meta-frontier",
            "journal-capture",
            "journal-frontier",
            "receipt-capture",
            "receipt-frontier",
        ] {
            let fixture = Fixture::new()?;
            if declared {
                fixture.declare(FIRST)?;
            }
            let (table, key) = match damage {
                "meta-capture" => (&fixture.store.meta, DECLARATION.to_string()),
                "meta-frontier" => (
                    &fixture.store.meta,
                    format!("{ENTRY_PREFIX}{}", digest("a")?),
                ),
                "journal-capture" => (
                    &fixture.store.journal,
                    "public_resource_capture/v1/forged".to_string(),
                ),
                "journal-frontier" => (
                    &fixture.store.journal,
                    format!("{ENTRY_PREFIX}{}", digest("a")?),
                ),
                "receipt-capture" => (
                    &fixture.store.receipts,
                    format!("{OPERATION_PREFIX}{}", digest("a")?),
                ),
                "receipt-frontier" => (
                    &fixture.store.receipts,
                    format!("{FRONTIER_OPERATION_PREFIX}{}", digest("a")?),
                ),
                _ => return Err("forgery case".into()),
            };
            let record =
                PublicResourceDeclarationV1::new(fixture.binding.clone(), timestamp(FIRST)?)?;
            let payload = if damage.starts_with("receipt-") {
                codec::encode(&record.receipt()?)?
            } else {
                codec::encode(&record)?
            };
            table.insert(key, payload)?;
            fixture.store.flush()?;
            let Fixture {
                directory,
                store,
                writer,
                ..
            } = fixture;
            let root = directory.path().join("store");
            drop((store, writer));
            cold_refused(&root)?;
        }
    }
    Ok(())
}

#[test]
fn retained_declaration_requires_exact_original_receipt_during_cold_open() -> TestResult {
    for damage in ["missing", "digest", "clock", "appended", "credited"] {
        let fixture = Fixture::new()?;
        let mut receipt = fixture.declare(FIRST)?.receipt().clone();
        match damage {
            "missing" => {
                fixture.store.receipts.remove(&receipt.operation)?;
            }
            "digest" => {
                receipt.digest =
                    different_digest(&AcquisitionDigest::parse(&receipt.digest)?)?.to_string();
            }
            "clock" => {
                receipt.at = timestamp(LATER)?.to_rfc3339();
            }
            "appended" => {
                receipt.appended = 1;
            }
            "credited" => {
                receipt.credited = true;
            }
            _ => return Err("receipt tamper case".into()),
        }
        if damage != "missing" {
            fixture
                .store
                .receipts
                .insert(&receipt.operation, codec::encode(&receipt)?)?;
        }
        fixture.store.flush()?;
        let Fixture {
            directory,
            store,
            writer,
            ..
        } = fixture;
        let root = directory.path().join("store");
        drop((store, writer));
        cold_refused(&root)?;
    }
    Ok(())
}

#[test]
fn generic_phase_operation_and_credit_paths_refuse_without_touching_owned_state() -> TestResult {
    let fixture = Fixture::new()?;
    let receipt = fixture.declare(FIRST)?.receipt().clone();
    let before = Witness::capture(&fixture.store)?;
    for phase in [
        "public_resource_capture",
        "public_resource_frontier",
        "public_resource_capture_v1",
        "public_resource_frontier_v1",
    ] {
        refused(
            fixture.store.journal_done(phase, "forged", &"replacement"),
            "public resource owned phase required",
        )?;
        refused(
            fixture
                .store
                .write_batch()
                .journal_done(phase, "forged", &"replacement"),
            "public resource owned phase required",
        )?;
    }
    for operation in [
        receipt.operation,
        format!("{FRONTIER_OPERATION_PREFIX}{}", digest("a")?),
    ] {
        refused(
            fixture
                .store
                .write_batch()
                .commit_once(&operation, "replacement"),
            "public resource owned operation required",
        )?;
        refused(
            fixture
                .store
                .credit_observations("controlled-endpoint", &operation),
            "public resource owned operation required",
        )?;
    }
    check!(eq; fixture.store.endpoint_credit("controlled-endpoint")?, 0);
    check!(eq; Witness::capture(&fixture.store)?, before);
    Ok(())
}

#[test]
fn derived_phase_and_publish_refusals_have_no_writes_after_normal_stage_reservation() -> TestResult
{
    let fixture = Fixture::new()?;
    let receipt = fixture.declare(FIRST)?.receipt().clone();
    for phase in ["public_resource_capture", "public_resource_frontier"] {
        let mut stage = fixture.store.stage_derived()?;
        let reserved = Witness::capture(&fixture.store)?;
        refused(
            stage.journal_done_once(phase, "forged", &"replacement"),
            "public resource owned phase required",
        )?;
        drop(stage);
        check!(eq; Witness::capture(&fixture.store)?, reserved);
    }
    for operation in [
        receipt.operation,
        format!("{FRONTIER_OPERATION_PREFIX}{}", digest("a")?),
    ] {
        let stage = fixture.store.stage_derived()?;
        let reserved = Witness::capture(&fixture.store)?;
        refused(
            stage.publish(&operation, "replacement"),
            "public resource owned operation required",
        )?;
        check!(eq; Witness::capture(&fixture.store)?, reserved);
    }
    Ok(())
}

#[test]
fn pruning_removes_an_ordinary_receipt_but_preserves_declaration_and_replay() -> TestResult {
    let fixture = Fixture::new()?;
    let receipt = fixture.declare(FIRST)?.receipt().clone();
    let expected = declaration(&fixture.store)?;
    fixture
        .store
        .write_batch()
        .commit_once("ordinary-declaration-control", "control")?;
    check!(fixture
        .store
        .receipt("ordinary-declaration-control")?
        .is_some());
    let pruned = fixture.store.prune_receipts(&ReplayFrontier {
        at: "2100-01-01".to_string(),
        witnesses: vec![],
    })?;
    check!(eq; pruned.removed, 1);
    check!(fixture
        .store
        .receipt("ordinary-declaration-control")?
        .is_none());
    check!(eq; fixture.store.receipt(&receipt.operation)?, Some(receipt.clone()));
    check!(eq; declaration(&fixture.store)?, expected);
    let before = Witness::capture(&fixture.store)?;
    check!(eq; fixture.declare(LATER)?, Application::Repeated(receipt));
    check!(eq; Witness::capture(&fixture.store)?, before);
    Ok(())
}
