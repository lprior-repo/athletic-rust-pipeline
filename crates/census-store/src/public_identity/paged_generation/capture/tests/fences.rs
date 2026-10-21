use super::*;
use census_domain::model::{CensusRun, GradYear, RunManifest, SchoolYear, StoreIdentity};
use census_domain::UsJurisdiction;

#[test]
fn each_input_counter_and_lineage_dimension_refuses_but_journal_churn_does_not() -> TestResult {
    let original = super::super::super::tests::capture()?;
    for index in 0..6 {
        let mut moved = original.clone();
        let counter = moved.0.table_revisions.get_mut(index).ok_or("counter")?;
        *counter = counter.checked_add(1).ok_or("counter overflow")?;
        check!(binding::require_inputs(&original, &moved).is_err());
    }
    let mut moved = original.clone();
    moved.0.derived_generation = moved
        .0
        .derived_generation
        .checked_add(1)
        .ok_or("generation")?;
    check!(binding::require_inputs(&original, &moved).is_err());
    let mut moved = original.clone();
    moved.0.store = StoreIdentity::parse(&"2".repeat(64))?;
    check!(binding::require_inputs(&original, &moved).is_err());
    let mut moved = original.clone();
    moved.0.format.lifecycle_revision = 2;
    check!(binding::require_inputs(&original, &moved).is_err());
    let mut moved = original.clone();
    moved.0.format.store_schema = 2;
    check!(binding::require_inputs(&original, &moved).is_err());
    let mut moved = original.clone();
    moved.0.format.key_format = 2;
    check!(binding::require_inputs(&original, &moved).is_err());
    let mut moved = original.clone();
    moved.0.run = super::super::super::wire::RunBinding::Bound {
        manifest: RunManifest {
            store_identity: original.0.store.as_str().into(),
            run: CensusRun::new(SchoolYear::new(2026).ok_or("season")?, 1).ok_or("run")?,
            cohort: GradYear::CO2027,
            jurisdictions: vec![UsJurisdiction::Michigan],
        },
    };
    check!(binding::require_inputs(&original, &moved).is_err());
    let mut churn = original.clone();
    churn.0.snapshot_sequence = churn.0.snapshot_sequence.checked_add(1).ok_or("sequence")?;
    churn.0.evidence_generation = churn
        .0
        .evidence_generation
        .checked_add(1)
        .ok_or("evidence")?;
    binding::require_inputs(&original, &churn)?;
    Ok(())
}

#[test]
fn physical_byte_ceiling_splits_pages_and_readback_preserves_padding() -> TestResult {
    let (_directory, store, writer) = enrolled()?;
    let mut batch = store.db.batch();
    for index in 0..17 {
        let (key, mut value) = raw(index)?;
        value.resize(MAX_RAW_VALUE_BYTES, b' ');
        batch.insert(&store.entities, key, value);
    }
    marks(&store, &mut batch, 17);
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    let outcome = writer.capture_raw_generation_v2(&store, &operation("byte-split")?)?;
    let retained = outcome.retained();
    let snapshot = store.snapshot();
    let seal = retained.verify(&snapshot)?;
    check!(eq;seal.declared_inventory().pages(), 2);
    let binding = retained
        .binding()
        .table_binding(AuthorityTableV2::Athletes)?;
    let table = seal.tables().first().ok_or("table")?;
    let table = snapshot.raw_generation_table(&table.object(), &binding)?;
    let block = table.blocks().first().ok_or("block")?;
    let block = snapshot.raw_generation_block(&block.object(), &binding)?;
    check!(eq;block.pages().first().ok_or("page")?.rows(), 15);
    let mut index = 0_u64;
    for reference in block.pages() {
        let page = snapshot.raw_generation_page(&reference.object(), &binding)?;
        for row in page.rows() {
            let (key, mut value) = raw(index)?;
            value.resize(MAX_RAW_VALUE_BYTES, b' ');
            check!(eq;row.physical_key(), key);
            check!(eq;row.raw_value(), value);
            index = index.checked_add(1).ok_or("row count")?;
        }
    }
    check!(eq;index, 17);
    Ok(())
}
