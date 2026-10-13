use super::*;
use census_store::{InputState, Table};

#[test]
fn old_view_public_dataset_stays_unbound_after_live_first_binding() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let identity = store.store_identity()?;
    let old = store.snapshot();
    let sequence = old.sequence();
    let source = old.tables_digest(&super::super::INPUT_TABLES)?;
    let bound = manifest(identity.as_str())?;
    store.bind_run(&bound)?;
    let school = fixtures::school("Observed After Binding");
    store.append(Table::Schools, &school)?;
    let dataset = ExportDataset::from_snapshot(&store.clone(), &old)?;
    check!(eq; dataset.lineage.run, None);
    check!(eq; dataset.lineage.cohort, None);
    check!(eq; dataset.lineage.snapshot_sequence, sequence);
    check!(eq; dataset.lineage.source_digest, source);
    check!(eq; dataset.schools.len(), 0);
    let current = ExportDataset::load(&store)?;
    check!(eq; current.lineage.run, Some(bound.run));
    check!(eq; current.lineage.cohort, Some(bound.cohort));
    check!(current.schools.contains_key(&school.id));
    check!(ne; current.lineage.source_digest, dataset.lineage.source_digest);
    check!(eq; crate::export::store_lineage(&store)?.run, Some(bound.run));
    Ok(())
}

enum Binding {
    Unbound,
    Bound,
}

fn readback(binding: Binding, job: &str) -> TestResult {
    let seeded = fixtures::seeded_store()?;
    let store = seeded.store;
    let identity = store.store_identity()?;
    let bound = match binding {
        Binding::Unbound => None,
        Binding::Bound => {
            let value = manifest(identity.as_str())?;
            store.bind_run(&value)?;
            Some(value)
        }
    };
    let expected = crate::export::store_lineage(&store)?;
    let root = super::super::freeze_v2(&store, job)?;
    let retained = root.manifest();
    check!(eq; retained.lineage.run, bound.as_ref().map(|value| value.run));
    check!(eq; retained.lineage.cohort, bound.as_ref().map(|value| value.cohort));
    check!(eq; retained.input_token.run_manifest_digest, bound.as_ref().map(serialized_digest).transpose()?);
    check!(eq; retained.lineage.source_digest, expected.source_digest);
    check!(eq; retained.lineage.snapshot_sequence, expected.snapshot_sequence);
    check!(eq; retained.input_token.snapshot_sequence, expected.snapshot_sequence);
    check!(eq; retained.lineage.export_job.as_deref(), Some(job));
    check!(eq; retained.input_token.input_root, expected.source_digest);
    retained.input_token.validate_against(&retained.lineage)?;
    let encoded = retained.encode()?;
    check!(eq; objects::read(&objects::directory(root.directory()), root.manifest_digest(), objects::MANIFEST_CEILING)?, encoded);
    check!(eq; serialized_digest(retained)?, root.manifest_digest());
    let record = store
        .get_input_record(root.attempt())?
        .ok_or("frozen input record")?;
    check!(eq; record.state(), InputState::Frozen);
    check!(eq; record.chunks().first().map(String::as_str), Some(root.manifest_digest()));
    drop(store);
    let reopened = Store::open(seeded.directory.path())?;
    let reused = super::super::freeze_v2(&reopened, job)?;
    check!(eq; reused, root);
    let dataset = super::super::to_export_dataset(&root, &reopened)?;
    let current = ExportDataset::load(&reopened)?;
    check!(eq; dataset.lineage, root.manifest().lineage);
    check!(eq; dataset.lineage.input_digest, current.lineage.input_digest);
    check!(eq; dataset.lineage.source_digest, current.lineage.source_digest);
    check!(eq; dataset.schools.len(), 1);
    check!(eq; dataset.athletes.len(), 2);
    check!(eq; reopened.get_input_record(root.attempt())?, Some(record));
    check!(eq; reopened.run_manifest()?, bound);
    Ok(())
}

#[test]
fn unbound_freeze_v2_readback_retains_the_fully_absent_binding() -> TestResult {
    readback(Binding::Unbound, "pinned-run:unbound-readback")
}

#[test]
fn bound_freeze_v2_readback_retains_the_full_ordered_manifest_digest() -> TestResult {
    readback(Binding::Bound, "pinned-run:bound-readback")
}
