use super::*;

#[test]
fn derived_rows_replace_in_place_and_never_move_the_append_counter() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .replace_many(Table::ReviewCases, &[derived("case:1", 1)])
        .unwrap();
    store
        .replace_many(Table::ReviewCases, &[derived("case:1", 2)])
        .unwrap();

    let scanned: Vec<DerivedRow> = store.scan(Table::ReviewCases).unwrap();
    assert_eq!(scanned.len(), 1, "one row per key whatever the pass count");
    assert_eq!(scanned[0].note, 2, "the later derivation replaces the row");
    assert_eq!(rows_held(&store, Table::ReviewCases), 1);
    assert_eq!(
        sequence_pointer(&store, Table::ReviewCases),
        0,
        "a derived write reserves nothing"
    );
    let stats = store.stats().unwrap();
    let appended = stats
        .appended
        .iter()
        .find(|(table, _)| table == "review_cases")
        .map(|(_, count)| *count)
        .unwrap();
    assert_eq!(appended, 0, "a replaced row is not an appended observation");
}

#[test]
fn a_rejected_derived_record_leaves_the_table_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let good = derived("case-1", 1);
    store
        .replace_many(Table::ReviewCases, std::slice::from_ref(&good))
        .unwrap();

    assert!(matches!(
        store.replace_many(Table::ReviewCases, &[derived("", 2)]),
        Err(StoreError::Invariant { .. })
    ));

    let scanned: Vec<DerivedRow> = store.scan(Table::ReviewCases).unwrap();
    assert_eq!(scanned.len(), 1);
    assert_eq!(scanned[0].id, good.id);
}

#[test]
fn a_derived_map_table_keys_one_row_per_id_under_sequence_zero() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .replace_many(
            Table::ReviewCases,
            &[derived("case:1", 1), derived("case:2", 1)],
        )
        .unwrap();
    store
        .replace_many(
            Table::ReviewCases,
            &[
                derived("case:1", 2),
                derived("case:2", 2),
                derived("case:2", 3),
            ],
        )
        .unwrap();

    let walk = store.walk_table(Table::ReviewCases).unwrap();
    assert_eq!(walk.rows, 2, "one physical row per entity key");
    assert_eq!(
        walk.highest_sequence,
        Some(0),
        "every row is keyed under sequence zero"
    );
    assert_eq!(walk.foreign_sequences, 0);
    assert_eq!(walk.repeated_ids, 0, "no id owns two rows");
    assert_eq!(rows_held(&store, Table::ReviewCases), 2);

    let notes: Vec<(String, u32)> = store
        .scan::<DerivedRow>(Table::ReviewCases)
        .unwrap()
        .into_iter()
        .map(|row| (row.id, row.note))
        .collect();
    assert_eq!(
        notes,
        vec![("case:1".to_string(), 2), ("case:2".to_string(), 3)],
        "the last record named for an id is the row that stands"
    );
}

#[test]
fn a_snapshot_write_replaces_the_rows_it_does_not_name() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .replace_many(Table::Coverage, &[derived("wi", 1), derived("oh", 1)])
        .unwrap();
    assert_eq!(store.scan::<DerivedRow>(Table::Coverage).unwrap().len(), 2);

    store
        .replace_many(Table::Coverage, &[derived("oh", 2)])
        .unwrap();
    let ids: Vec<String> = store
        .scan::<DerivedRow>(Table::Coverage)
        .unwrap()
        .into_iter()
        .map(|row| row.id)
        .collect();
    assert_eq!(
        ids,
        vec!["oh".to_string()],
        "the row the batch does not name is gone, not merely unread"
    );
    assert_eq!(rows_held(&store, Table::Coverage), 1);

    store
        .replace_many(Table::Coverage, &[derived("wi", 3)])
        .unwrap();
    assert_eq!(
        store.scan::<DerivedRow>(Table::Coverage).unwrap().len(),
        1,
        "and the next snapshot replaces what that one left"
    );
    assert_eq!(rows_held(&store, Table::Coverage), 1);
}

#[test]
fn an_empty_snapshot_write_empties_the_table() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    store
        .replace_many(Table::Coverage, &[derived("wi", 1), derived("oh", 1)])
        .unwrap();
    assert_eq!(rows_held(&store, Table::Coverage), 2);

    store
        .replace_many(Table::Coverage, &[] as &[DerivedRow])
        .unwrap();
    assert!(
        store
            .scan::<DerivedRow>(Table::Coverage)
            .unwrap()
            .is_empty(),
        "a snapshot derivation that found nothing leaves no row behind"
    );
    assert_eq!(rows_held(&store, Table::Coverage), 0);

    store
        .replace_many(Table::ReviewCases, &[derived("case:1", 5)])
        .unwrap();
    store
        .replace_many(Table::ReviewCases, &[] as &[DerivedRow])
        .unwrap();
    assert_eq!(
        rows_held(&store, Table::ReviewCases),
        1,
        "a map table keeps the row an empty batch does not name"
    );
}

#[test]
fn a_derived_write_clears_the_foreign_sequences_of_the_ids_it_names() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let imported = derived("case:1", 7);
    let mut batch = store.db.batch();
    batch.insert(
        &store.entities,
        observation_key(Table::ReviewCases, "case:1", 7),
        serde_json::to_vec(&imported).unwrap(),
    );
    batch.commit().unwrap();
    assert_eq!(
        store
            .walk_table(Table::ReviewCases)
            .unwrap()
            .foreign_sequences,
        1,
        "the fixture is a row the store's own writer would never have keyed"
    );

    store
        .replace_many(Table::ReviewCases, &[derived("case:1", 8)])
        .unwrap();

    let walk = store.walk_table(Table::ReviewCases).unwrap();
    assert_eq!(walk.rows, 1, "the copy is gone, not merely outranked");
    assert_eq!(walk.foreign_sequences, 0);
    let rows = store.scan::<DerivedRow>(Table::ReviewCases).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].note, 8,
        "the derivation, not the copy it replaced, is what reads"
    );
}

#[test]
fn an_over_bound_replace_batch_is_refused_before_a_single_row_is_encoded() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let records = vec![(); usize::try_from(MAX_ROWS_PER_TABLE).unwrap_or(usize::MAX) + 1];

    match store.replace_many(Table::Coverage, &records) {
        Err(StoreError::TooManyRows { table, max }) => {
            assert_eq!(table, "coverage");
            assert_eq!(
                max,
                usize::try_from(MAX_ROWS_PER_TABLE).unwrap_or(usize::MAX)
            );
        }
        other => panic!("expected the row ceiling to refuse the batch, got {other:?}"),
    }
    assert!(store
        .scan::<CoverageRow>(Table::Coverage)
        .unwrap()
        .is_empty());
    assert_eq!(rows_held(&store, Table::Coverage), 0);
}
