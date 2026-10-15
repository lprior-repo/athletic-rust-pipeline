use census_domain::model::StoreIdentity;

use super::support::refused;
use super::*;

#[derive(Clone, Copy)]
enum GenericLineageWrite {
    Direct,
    Batch,
}

fn refuses_lineage_replacement(route: GenericLineageWrite) -> TestResult {
    let (directory, store, writer) = fixture()?;
    let athlete = subject("lineage-guard", "701")?;
    store.append(Table::Athletes, &athlete)?;
    let committed = writer.establish_source_bound(
        &store,
        "retained-before-lineage-write",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let identity = store.store_identity()?;
    let foreign = StoreIdentity::parse(&"b".repeat(64))?;
    check!(ne; identity, foreign);
    let before = retained_state(&store)?;
    let error = match route {
        GenericLineageWrite::Direct => {
            refused(store.journal_done("export-lineage-v1", "store-identity", &foreign))?
        }
        GenericLineageWrite::Batch => {
            let mut batch = store.write_batch();
            let error =
                refused(batch.journal_done("export-lineage-v1", "store-identity", &foreign))?;
            check!(batch.is_empty());
            batch.commit()?;
            error
        }
    };
    check!(matches!(
        error,
        StoreError::PublicIdentity(crate::PublicIdentityError::ReservedWrite)
    ));
    check!(eq; retained_state(&store)?, before);
    check!(eq; store.store_identity()?, identity);
    check!(eq; writer.head(&store)?, *committed.head());
    drop(writer);
    drop(store);
    let root = directory.path().join("identity");
    check!(eq; physical_state(&root)?, before);
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    check!(eq; retained_state(&store)?, before);
    check!(eq; store.store_identity()?, identity);
    check!(eq; writer.head(&store)?, *committed.head());
    let repeated = writer.establish_source_bound(
        &store,
        "retained-before-lineage-write",
        &athlete.id.cast(),
        store.snapshot().revision(),
        committed.head(),
    )?;
    check!(repeated.replayed());
    check!(eq; repeated.public_id(), committed.public_id());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn public_identity_direct_lineage_replacement_is_refused_without_mutation() -> TestResult {
    refuses_lineage_replacement(GenericLineageWrite::Direct)
}

#[test]
fn public_identity_batch_lineage_replacement_is_refused_without_mutation() -> TestResult {
    refuses_lineage_replacement(GenericLineageWrite::Batch)
}
