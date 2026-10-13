use super::*;
use census_domain::model::{ProfileId, ProfileSelectionV1};

#[test]
fn oversized_lineage_refuses_all_selected_reads_without_replacement() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open_with(dir.path(), 8_388_608)?;
    let payload = contract(7)?;
    let reference = payload.reference()?;
    equal(
        store.register_region_contract(&reference, &payload)?,
        RegionContractRegistration::Registered(reference.clone()),
    )?;
    let identity = StoreIdentity::parse(&"A".repeat(64))?;
    let selection = ProfileSelectionV1::new(identity.clone(), ProfileId::parse(&"b".repeat(64))?);
    let key = Store::journal_key("export-lineage-v1", "store-identity");
    let count = crate::MAX_JOURNAL_VALUE_BYTES
        .checked_add(1)
        .ok_or("lineage ceiling")?;
    let bytes = vec![b'{'; count];
    {
        let _appends = store.lock_appends();
        let mut batch = store.db.batch();
        batch.insert(&store.journal, key, bytes);
        batch.durability(Some(PersistMode::SyncData)).commit()?;
    }
    let view = store.snapshot();
    let before = Witness::capture(&store)?;
    for result in [
        view.store_identity().map(|_| ()),
        store.store_identity().map(|_| ()),
        store.selected_profile(&selection).map(|_| ()),
        view.selected_profile(&selection).map(|_| ()),
        store
            .selected_region_contract(&identity, &reference)
            .map(|_| ()),
        view.selected_region_contract(&identity, &reference)
            .map(|_| ()),
    ] {
        match result {
            Err(StoreError::StoreLineage(crate::StoreLineageError::OversizedEnvelope {
                bytes,
                max,
            })) => {
                equal(bytes, count)?;
                equal(max, crate::MAX_JOURNAL_VALUE_BYTES)?;
            }
            other => {
                return Err(format!("oversized lineage was decoded or replaced: {other:?}").into())
            }
        }
    }
    equal(Witness::capture(&store)?, before)
}
