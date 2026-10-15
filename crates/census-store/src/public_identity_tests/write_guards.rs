use crate::PublicIdentityError;

use super::support::{accepted, refused};
use super::*;

#[test]
fn public_identity_generic_identity_application_and_replacements_are_refused() -> TestResult {
    let (_directory, store, _) = fixture()?;
    let athlete = subject("generic", "110")?;
    store.append(Table::Athletes, &athlete)?;
    let decision = accepted(&athlete)?;
    let before = retained_state(&store)?;
    let error = refused(store.replace(Table::AthleteIdentityDecisions, decision.record()))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::ReservedWrite)
    ));
    check!(eq; retained_state(&store)?, before);
    let error = refused(store.replace_many(
        Table::AthleteIdentityDecisions,
        std::slice::from_ref(decision.record()),
    ))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::ReservedWrite)
    ));
    check!(eq; retained_state(&store)?, before);
    let mut batch = store.write_batch();
    let error = refused(batch.replace_many(
        Table::AthleteIdentityDecisions,
        std::slice::from_ref(decision.record()),
    ))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::ReservedWrite)
    ));
    check!(batch.is_empty());
    drop(batch);
    let error = refused(store.apply_identity_decisions(&[decision]))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::ReservedWrite)
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_identity_generic_journal_writes_cannot_enter_reserved_namespace() -> TestResult {
    let (_directory, store, _) = fixture()?;
    let before = retained_state(&store)?;
    for phase in [
        "public_identity:v1",
        "public_identity:v1:history",
        "public_identity:v2",
    ] {
        let error = refused(store.journal_done(phase, "forged", &"unowned"))?;
        check!(matches!(
            error,
            StoreError::PublicIdentity(PublicIdentityError::ReservedWrite)
        ));
        let mut batch = store.write_batch();
        let error = refused(batch.journal_done(phase, "forged", &"unowned"))?;
        check!(matches!(
            error,
            StoreError::PublicIdentity(PublicIdentityError::ReservedWrite)
        ));
        check!(batch.is_empty());
    }
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_identity_generic_receipts_cannot_forge_owned_operation_results() -> TestResult {
    let (_directory, store, _) = fixture()?;
    let before = retained_state(&store)?;
    let error = refused(
        store
            .write_batch()
            .commit_once("public_identity_v1:forged", "payload"),
    )?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::ReservedWrite)
    ));
    let error = refused(store.write_batch().commit_once_at_revision(
        "public_identity_v1:forged",
        "payload",
        store.snapshot().revision(),
    ))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(PublicIdentityError::ReservedWrite)
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
