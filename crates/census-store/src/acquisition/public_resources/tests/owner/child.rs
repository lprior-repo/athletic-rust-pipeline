use super::*;
use crate::acquisition::public_resources::child as protocol;
use crate::acquisition::public_resources::{
    PublicDirectoryEvidenceV1, PublicDirectoryRosterAssessmentV1, PublicDirectorySemanticContractV1,
};
use census_domain::roster_affiliation_authority::SourceRowLocator;

mod capacity;
mod cold;
mod fixture;
mod intake;
mod recovery;
use fixture::{assessment, retained};

#[test]
fn controlled_assessment_of_owned_directory_registers_child_once_and_replays_exactly() -> TestResult
{
    let f = Fixture::new()?;
    let (owner, original) = retained(&f)?;
    let assessed = assessment(&f.store, &owner, &original)?;
    let before = Witness::capture(&f.store)?;
    let prepared = f.store.prepare_public_roster_child(&assessed)?;
    let work = prepared.work().clone();
    check!(eq; Witness::capture(&f.store)?, before);
    check!(eq; work.request().original_url(), assessed.roster_url());
    check!(eq; work.slot().store_identity(), f.writer.store_identity());
    check!(eq; work.slot().owner_key(), owner.scope().owner_key());
    let owned = f.store.register_public_roster_child(prepared)?;
    let after = Witness::capture(&f.store)?;
    check!(eq; after.sequence, before.sequence.checked_add(1).ok_or("child sequence")?);
    check!(eq; after.journal.len(), before.journal.len() + 3);
    check!(eq; after.receipts.len(), before.receipts.len() + 1);
    check!(eq; after.meta, before.meta);
    check!(eq; after.entities, before.entities);
    fixture::verify_registered(&f.store, &work)?;
    check!(eq; f.store.inspect_acquisition_work(&owned, &mut AcquisitionReadBudget::for_work())?, crate::AcquireDecision::Ready);
    let repeated = f
        .store
        .register_public_roster_child(f.store.prepare_public_roster_child(&assessed)?)?;
    check!(eq; repeated.work(), &work);
    check!(eq; repeated.work_id(), owned.work_id());
    check!(eq; Witness::capture(&f.store)?, after);
    let original_after =
        owner.directory_evidence(&f.store, &original, &mut AcquisitionReadBudget::for_work())?;
    check!(eq; original_after.retained().reference(), &original);
    check!(eq; original_after.retained().body_reference(), &original);
    check!(eq; Witness::capture(&f.store)?, after);
    Ok(())
}

#[test]
fn foreign_reference_owner_and_bad_terminal_digest_refuse_without_writing_either_store(
) -> TestResult {
    let f = Fixture::new()?;
    let (owner, original) = retained(&f)?;
    let foreign = Fixture::new()?;
    let (other, foreign_original) = retained(&foreign)?;
    let assessed = assessment(&foreign.store, &other, &foreign_original)?;
    let before = Witness::capture(&f.store)?;
    let foreign_before = Witness::capture(&foreign.store)?;
    check!(owner
        .directory_evidence(
            &f.store,
            &foreign_original,
            &mut AcquisitionReadBudget::for_work()
        )
        .is_err());
    check!(other
        .directory_evidence(&f.store, &original, &mut AcquisitionReadBudget::for_work())
        .is_err());
    check!(f.store.prepare_public_roster_child(&assessed).is_err());
    let bad = AcquisitionOccurrenceRefV1::new(
        original.attempt().clone(),
        different_digest(original.terminal_digest())?,
    );
    check!(owner
        .directory_evidence(&f.store, &bad, &mut AcquisitionReadBudget::for_work())
        .is_err());
    check!(eq; Witness::capture(&f.store)?, before);
    check!(eq; Witness::capture(&foreign.store)?, foreign_before);
    Ok(())
}

#[test]
fn changed_roster_target_and_outside_parent_locators_refuse_without_child_rows() -> TestResult {
    let f = Fixture::new()?;
    let (owner, original) = retained(&f)?;
    let before = Witness::capture(&f.store)?;
    for change in fixture::assessment_changes() {
        let mut assessed = assessment(&f.store, &owner, &original)?;
        change(&mut assessed);
        check!(f.store.prepare_public_roster_child(&assessed).is_err());
        check!(eq; Witness::capture(&f.store)?, before);
    }
    Ok(())
}

#[test]
fn changed_protocol_witness_cannot_rebind_existing_child_even_with_same_work() -> TestResult {
    let f = Fixture::new()?;
    let (owner, original) = retained(&f)?;
    let assessed = assessment(&f.store, &owner, &original)?;
    let owned = f
        .store
        .register_public_roster_child(f.store.prepare_public_roster_child(&assessed)?)?;
    let before = Witness::capture(&f.store)?;
    let mut changed = assessment(&f.store, &owner, &original)?;
    changed.row_digest = different_digest(&changed.row_digest)?;
    let changed = f.store.prepare_public_roster_child(&changed)?;
    check!(eq; changed.work(), owned.work());
    refused(
        f.store.register_public_roster_child(changed),
        "public roster child witness rebound",
    )?;
    check!(eq; Witness::capture(&f.store)?, before);
    Ok(())
}

#[test]
fn prepared_child_rechecks_original_parent_before_registration() -> TestResult {
    let f = Fixture::new()?;
    let (owner, original) = retained(&f)?;
    let assessed = assessment(&f.store, &owner, &original)?;
    let prepared = f.store.prepare_public_roster_child(&assessed)?;
    let work = prepared.work().clone();
    std::fs::write(
        assessed.evidence().retained().body_path(),
        b"changed controlled parent",
    )?;
    let before = Witness::capture(&f.store)?;
    check!(f.store.register_public_roster_child(prepared).is_err());
    check!(eq; Witness::capture(&f.store)?, before);
    check!(!f.store.journal.contains_key(protocol::key(&work)?)?);
    Ok(())
}

#[test]
fn directory_count_is_a_bound_on_registered_children_including_the_new_child() -> TestResult {
    let f = Fixture::new()?;
    let (owner, original) = retained(&f)?;
    let first = assessment(&f.store, &owner, &original)?;
    f.store
        .register_public_roster_child(f.store.prepare_public_roster_child(&first)?)?;
    let mut second = assessment(&f.store, &owner, &original)?;
    second.team_id = "20006".into();
    second.team_url = "https://mi.milesplit.com/teams/20006-controlled-protocol".into();
    second.roster_url = format!("{}/roster", second.team_url);
    let prepared = f.store.prepare_public_roster_child(&second)?;
    let before = Witness::capture(&f.store)?;
    refused(
        f.store.register_public_roster_child(prepared),
        "public roster child directory inventory exhausted",
    )?;
    check!(eq; Witness::capture(&f.store)?, before);
    Ok(())
}
