use census_domain::model::StoreIdentity;

use super::support::refused;
use super::*;

fn rejects_generic_lineage_entry(phase: &str, key: &str) -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let identity = store.store_identity()?;
    let head = writer.head(&store)?;
    let foreign = StoreIdentity::parse(&"c".repeat(64))?;
    check!(ne; identity, foreign);
    let before = retained_state(&store)?;
    let error = refused(store.journal_done(phase, key, &foreign))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(crate::PublicIdentityError::ReservedWrite)
    ));
    check!(eq; retained_state(&store)?, before);
    let mut batch = store.write_batch();
    let error = refused(batch.journal_done(phase, key, &foreign))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(crate::PublicIdentityError::ReservedWrite)
    ));
    check!(batch.is_empty());
    batch.commit()?;
    check!(eq; retained_state(&store)?, before);
    check!(eq; store.store_identity()?, identity);
    check!(eq; writer.head(&store)?, head);
    Ok(())
}

#[test]
fn public_identity_generic_lineage_sibling_is_refused_without_mutation() -> TestResult {
    rejects_generic_lineage_entry("export-lineage-v1", "unowned-sibling")
}

#[test]
fn public_identity_generic_lineage_nul_descendant_is_refused_without_mutation() -> TestResult {
    rejects_generic_lineage_entry("export-lineage-v1\0store-identity", "unowned-descendant")
}

#[test]
fn public_identity_derived_stage_lineage_entries_are_refused_after_generation_reservation(
) -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let identity = store.store_identity()?;
    let head = writer.head(&store)?;
    let foreign = StoreIdentity::parse(&"d".repeat(64))?;
    check!(ne; identity, foreign);
    let before_stage = retained_state(&store)?;
    let mut stage = store.stage_derived()?;
    let after_reservation = retained_state(&store)?;
    check!(ne; after_reservation, before_stage);
    for (phase, key) in [
        ("export-lineage-v1", "store-identity"),
        ("export-lineage-v1", "unowned-sibling"),
        ("export-lineage-v1\0store-identity", "unowned-descendant"),
    ] {
        let error = refused(stage.journal_done_once(phase, key, &foreign))?;
        check!(matches!(
            error,
            StoreError::PublicIdentity(crate::PublicIdentityError::ReservedWrite)
        ));
        check!(eq; retained_state(&store)?, after_reservation);
    }
    drop(stage);
    check!(eq; retained_state(&store)?, after_reservation);
    check!(eq; store.store_identity()?, identity);
    check!(eq; writer.head(&store)?, head);
    Ok(())
}
