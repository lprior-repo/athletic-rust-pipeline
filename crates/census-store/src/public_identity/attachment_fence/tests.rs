use super::*;
use crate::{Store, StoreError};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn raw_head_fence_is_pinned_without_claiming_accepted_identity() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let first = RegistryHead {
        sequence: 1,
        digest: "a".repeat(64),
    };
    let second = RegistryHead {
        sequence: 2,
        digest: "b".repeat(64),
    };
    store.meta.insert(HEAD, codec::encode(&first, 1024)?)?;
    let held = store.snapshot();
    store.meta.insert(HEAD, codec::encode(&second, 1024)?)?;
    check!(eq; held.attachment_public_head()?, first);
    check!(eq; store.snapshot().attachment_public_head()?, second);
    Ok(())
}

#[test]
fn missing_and_oversized_head_keep_existing_typed_refusals() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    check!(matches!(
        store.snapshot().attachment_public_head(),
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "missing head"
        )))
    ));
    store.meta.insert(HEAD, vec![b'x'; 1025])?;
    check!(matches!(
        store.snapshot().attachment_public_head(),
        Err(StoreError::RowTooLarge {
            bytes: 1025,
            max: 1024
        })
    ));
    Ok(())
}

#[test]
fn duplicate_head_fields_are_not_a_successful_fence() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    store
        .meta
        .insert(HEAD, br#"{"sequence":1,"sequence":2,"digest":"a"}"#)?;
    check!(matches!(
        store.snapshot().attachment_public_head(),
        Err(StoreError::Decode { .. })
    ));
    Ok(())
}
