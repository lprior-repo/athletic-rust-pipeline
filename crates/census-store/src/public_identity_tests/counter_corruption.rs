use super::support::refused;
use super::*;

fn committed_fixture() -> TestResult<(tempfile::TempDir, Store)> {
    let (directory, store, writer) = fixture()?;
    let athlete = subject("counter-corruption", "702")?;
    store.append(Table::Athletes, &athlete)?;
    writer.establish_source_bound(
        &store,
        "counter-corruption",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    check!(eq; store.count(Table::Athletes)?, 1);
    check!(eq; store.count(Table::AthleteIdentityDecisions)?, 1);
    Ok((directory, store))
}

fn refuses_corrupt_counter(directory: tempfile::TempDir, store: Store) -> TestResult {
    store.flush()?;
    let before = retained_state(&store)?;
    drop(store);
    let root = directory.path().join("identity");
    check!(eq; physical_state(&root)?, before);
    let error = refused(Store::open_public_identity_qualification_v1(&root))?;
    check!(matches!(
        error,
        StoreError::PublicIdentity(crate::PublicIdentityError::Corrupt(_))
    ));
    check!(eq; physical_state(&root)?, before);
    Ok(())
}

#[test]
fn public_identity_sequence_rollback_refuses_before_reusing_an_observation_key() -> TestResult {
    let (directory, store) = committed_fixture()?;
    let table = Table::Athletes;
    check!(eq; table.storage_mode(), crate::StorageMode::ObservationLog);
    let key = crate::keys::observation_prefix(table);
    let mut rows = store.entities.prefix(key);
    let physical = rows.next().ok_or("missing athlete observation")?.key()?;
    check!(rows.next().is_none());
    let (stored_table, _, sequence) =
        crate::keys::view_observation_key(&physical).ok_or("invalid athlete observation key")?;
    check!(eq; stored_table, table.file().as_bytes());
    check!(eq; sequence, 0);
    check!(eq; crate::meta::get_u64(&store.meta, "sequence:athletes")?, Some(1));
    drop(rows);
    store.meta.insert("sequence:athletes", b"0")?;
    refuses_corrupt_counter(directory, store)
}

#[test]
fn public_identity_physical_row_count_mismatch_refuses_without_repairing_marks() -> TestResult {
    for table in [Table::Athletes, Table::AthleteIdentityDecisions] {
        for wrong_count in ["0", "2"] {
            let (directory, store) = committed_fixture()?;
            let physical = store
                .entities
                .prefix(crate::keys::observation_prefix(table))
                .take(2)
                .map(|guard| guard.key())
                .collect::<Result<Vec<_>, _>>()?;
            check!(eq; physical.len(), 1);
            if table.storage_mode() == crate::StorageMode::DerivedMap {
                let key = physical.first().ok_or("missing identity decision")?;
                let (_, _, sequence) = crate::keys::view_observation_key(key)
                    .ok_or("invalid identity decision key")?;
                check!(eq; sequence, crate::keys::DERIVED_SEQUENCE);
                check!(eq; crate::meta::get_u64(
                    &store.meta,
                    &format!("sequence:{}", table.file()),
                )?, Some(0));
            }
            store
                .meta
                .insert(format!("rows:{}", table.file()), wrong_count.as_bytes())?;
            refuses_corrupt_counter(directory, store)?;
        }
    }
    Ok(())
}
