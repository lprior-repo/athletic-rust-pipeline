use super::*;
use crate::public_identity_tests::support::{physical_state, retained_state, TestResult};

#[test]
fn graph_qualification_creates_lifecycle_seven_without_implicit_capabilities() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("current");
    let declaration = Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    check!(eq; store.store_format()?.lifecycle_revision, Some(7));
    check!(eq; writer.store_identity(), declaration.store_identity());
    check!(eq; writer.qualification_version(), PublicIdentityQualificationVersion::RetainedAuthorityGraphV3);
    writer.require_current_qualification()?;
    check!(eq; store.snapshot().public_identity_input_revisions_v2()?, crate::PublicIdentityInputRevisionsV2::Disabled);
    check!(eq; writer.head(&store)?, genesis(declaration.store_identity())?);
    let before = retained_state(&store)?;
    drop(writer);
    drop(store);
    check!(Store::open_public_identity_qualification_v1(&root).is_err());
    check!(Store::open_public_identity_qualification_v2(&root).is_err());
    check!(Store::replay_historical_public_identity(&root).is_err());
    check!(Store::open_existing(&root).is_err());
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    check!(eq; retained_state(&store)?, before);
    check!(eq; writer.head(&store)?, genesis(declaration.store_identity())?);
    Ok(())
}

#[test]
fn historical_families_replay_without_a_mutating_writer_or_reset() -> TestResult {
    [
        PublicIdentityQualificationVersion::SourceBoundV1,
        PublicIdentityQualificationVersion::RetainedAuthorityContextV2,
    ]
    .into_iter()
    .try_for_each(|version| {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("historical");
        let declaration = create(&root, version)?;
        let before = physical_state(&root)?;
        let accepted = Store::replay_historical_public_identity(&root)?;
        check!(eq; accepted.qualification_version(), version);
        check!(eq; accepted.head(), &genesis(declaration.store_identity())?);
        check!(Store::open_public_identity_qualification_v1(&root).is_err());
        check!(Store::open_public_identity_qualification_v2(&root).is_err());
        check!(Store::open_public_identity_qualification_v3(&root).is_err());
        check!(Store::create_public_identity_qualification_v3(&root).is_err());
        check!(eq; physical_state(&root)?, before);
        Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
    })
}

#[test]
fn historical_writer_routes_refuse_before_creating_a_path() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("absent");
    check!(Store::create_public_identity_qualification_v1(&root).is_err());
    check!(Store::create_public_identity_qualification_v2(&root).is_err());
    check!(Store::open_public_identity_qualification_v1(&root).is_err());
    check!(Store::open_public_identity_qualification_v2(&root).is_err());
    check!(!root.exists());
    Ok(())
}

#[test]
fn graph_open_does_not_create_and_creation_does_not_adopt_artifacts() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("absent");
    check!(Store::open_public_identity_qualification_v3(&root).is_err());
    check!(!root.exists());
    std::fs::create_dir(&root)?;
    let artifact = root.join("historical.body");
    std::fs::write(&artifact, b"retained evidence")?;
    check!(matches!(
        Store::create_public_identity_qualification_v3(&root),
        Err(StoreError::PublicIdentity(
            PublicIdentityError::ExistingRoot
        ))
    ));
    check!(eq; std::fs::read(&artifact)?, b"retained evidence");
    check!(eq; std::fs::read_dir(&root)?.count(), 1);
    Ok(())
}

#[test]
fn qualification_lifecycle_classification_includes_current_graph_and_historical_families() {
    for revision in [5, 6, 7] {
        check!(crate::public_identity::is_qualification_lifecycle(Some(
            revision
        )));
    }
    for revision in [None, Some(0), Some(4), Some(8), Some(u32::MAX)] {
        check!(!crate::public_identity::is_qualification_lifecycle(
            revision
        ));
    }
}

#[test]
fn reopened_graph_store_keeps_generic_identity_writes_reserved() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("current");
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    drop(writer);
    drop(store);
    let (store, _writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    let before = retained_state(&store)?;
    check!(matches!(
        store.replace_many::<serde_json::Value>(Table::AthleteIdentityDecisions, &[]),
        Err(StoreError::PublicIdentity(
            PublicIdentityError::ReservedWrite
        ))
    ));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}
