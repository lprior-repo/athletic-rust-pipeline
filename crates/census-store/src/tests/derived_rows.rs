use super::*;

#[test]
fn derived_rows_replace_in_place_and_never_move_the_append_counter() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.replace_many(Table::ReviewCases, &[derived("case:1", 1)])?;
    store.replace_many(Table::ReviewCases, &[derived("case:1", 2)])?;
    let scanned: Vec<DerivedRow> = store.scan(Table::ReviewCases)?;
    {
        let (left, right) = (&scanned.len(), &1);
        if left != right {
            return Err(format!(
                "one row per key whatever the pass count — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    {
        let (left, right) = (&scanned[0].note, &2);
        if left != right {
            return Err(format!(
                "the later derivation replaces the row — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    {
        let (left, right) = (&rows_held(&store, Table::ReviewCases)?, &1);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&sequence_pointer(&store, Table::ReviewCases)?, &0);
        if left != right {
            return Err(format!(
                "a derived write reserves nothing — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    let stats = store.stats()?;
    let appended = stats
        .appended
        .iter()
        .find(|(table, _)| table == "review_cases")
        .map(|(_, count)| *count)
        .ok_or("review_cases append counter")?;
    {
        let (left, right) = (&appended, &0);
        if left != right {
            return Err(format!(
                "a replaced row is not an appended observation — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    Ok(())
}

#[test]
fn a_rejected_derived_record_leaves_the_table_untouched() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let good = derived("case-1", 1);
    store.replace_many(Table::ReviewCases, std::slice::from_ref(&good))?;
    let outcome = store.replace_many(Table::ReviewCases, &[derived("", 2)]);
    if !matches!(outcome, Err(StoreError::Invariant { .. })) {
        return Err(format!("expected invariant refusal: {outcome:?}").into());
    }
    let scanned: Vec<DerivedRow> = store.scan(Table::ReviewCases)?;
    {
        let (left, right) = (&scanned.len(), &1);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&scanned[0].id, &good.id);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    Ok(())
}

#[test]
fn a_derived_map_table_keys_one_row_per_id_under_sequence_zero() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.replace_many(
        Table::ReviewCases,
        &[derived("case:1", 1), derived("case:2", 1)],
    )?;
    store.replace_many(
        Table::ReviewCases,
        &[
            derived("case:1", 2),
            derived("case:2", 2),
            derived("case:2", 3),
        ],
    )?;
    let walk = store.walk_table(Table::ReviewCases)?;
    {
        let (left, right) = (&walk.rows, &2);
        if left != right {
            return Err(
                format!("one physical row per entity key — left={left:?} right={right:?}").into(),
            );
        }
    }
    {
        let (left, right) = (&walk.highest_sequence, &Some(0));
        if left != right {
            return Err(format!(
                "every row is keyed under sequence zero — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    {
        let (left, right) = (&walk.foreign_sequences, &0);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&walk.repeated_ids, &0);
        if left != right {
            return Err(format!("no id owns two rows — left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&rows_held(&store, Table::ReviewCases)?, &2);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    let notes: Vec<(String, u32)> = store
        .scan::<DerivedRow>(Table::ReviewCases)?
        .into_iter()
        .map(|row| (row.id, row.note))
        .collect();
    {
        let (left, right) = (
            &notes,
            &vec![("case:1".to_string(), 2), ("case:2".to_string(), 3)],
        );
        if left != right {
            return Err(format!("the last record named for an id is the row that stands — left={left:?} right={right:?}").into());
        }
    }
    Ok(())
}

#[test]
fn a_snapshot_write_replaces_the_rows_it_does_not_name() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.replace_many(Table::Coverage, &[derived("wi", 1), derived("oh", 1)])?;
    {
        let (left, right) = (&store.scan::<DerivedRow>(Table::Coverage)?.len(), &2);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    store.replace_many(Table::Coverage, &[derived("oh", 2)])?;
    let ids: Vec<String> = store
        .scan::<DerivedRow>(Table::Coverage)?
        .into_iter()
        .map(|row| row.id)
        .collect();
    {
        let (left, right) = (&ids, &vec!["oh".to_string()]);
        if left != right {
            return Err(format!("the row the batch does not name is gone, not merely unread — left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&rows_held(&store, Table::Coverage)?, &1);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    store.replace_many(Table::Coverage, &[derived("wi", 3)])?;
    {
        let (left, right) = (&store.scan::<DerivedRow>(Table::Coverage)?.len(), &1);
        if left != right {
            return Err(format!(
                "and the next snapshot replaces what that one left — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    {
        let (left, right) = (&rows_held(&store, Table::Coverage)?, &1);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    Ok(())
}

#[test]
fn an_empty_snapshot_write_empties_the_table() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.replace_many(Table::Coverage, &[derived("wi", 1), derived("oh", 1)])?;
    {
        let (left, right) = (&rows_held(&store, Table::Coverage)?, &2);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    store.replace_many(Table::Coverage, &[] as &[DerivedRow])?;
    let rows = store.scan::<DerivedRow>(Table::Coverage)?;
    if !rows.is_empty() {
        return Err(format!(
            "a snapshot derivation that found nothing leaves no row behind: {rows:?}"
        )
        .into());
    }
    {
        let (left, right) = (&rows_held(&store, Table::Coverage)?, &0);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    store.replace_many(Table::ReviewCases, &[derived("case:1", 5)])?;
    store.replace_many(Table::ReviewCases, &[] as &[DerivedRow])?;
    {
        let (left, right) = (&rows_held(&store, Table::ReviewCases)?, &1);
        if left != right {
            return Err(format!("a map table keeps the row an empty batch does not name — left={left:?} right={right:?}").into());
        }
    }
    Ok(())
}

#[test]
fn a_derived_write_clears_the_foreign_sequences_of_the_ids_it_names() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let imported = derived("case:1", 7);
    let mut batch = store.db.batch();
    batch.insert(
        &store.entities,
        observation_key(Table::ReviewCases, "case:1", 7),
        serde_json::to_vec(&imported)?,
    );
    batch.commit()?;
    {
        let (left, right) = (&store.walk_table(Table::ReviewCases)?.foreign_sequences, &1);
        if left != right {
            return Err(format!("the fixture is a row the store's own writer would never have keyed — left={left:?} right={right:?}").into());
        }
    }
    store.replace_many(Table::ReviewCases, &[derived("case:1", 8)])?;
    let walk = store.walk_table(Table::ReviewCases)?;
    {
        let (left, right) = (&walk.rows, &1);
        if left != right {
            return Err(format!(
                "the copy is gone, not merely outranked — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    {
        let (left, right) = (&walk.foreign_sequences, &0);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    let rows = store.scan::<DerivedRow>(Table::ReviewCases)?;
    {
        let (left, right) = (&rows.len(), &1);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&rows[0].note, &8);
        if left != right {
            return Err(format!("the derivation, not the copy it replaced, is what reads — left={left:?} right={right:?}").into());
        }
    }
    Ok(())
}

#[test]
fn an_over_bound_replace_batch_is_refused_before_a_single_row_is_encoded() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let records =
        vec![(); usize::try_from(MAX_ROWS_PER_TABLE).map_or(usize::MAX, |value| value) + 1];
    match store.replace_many(Table::Coverage, &records) {
        Err(StoreError::TooManyRows { table, max }) => {
            {
                let (left, right) = (&table, &"coverage");
                if left != right {
                    return Err(format!("left={left:?} right={right:?}").into());
                }
            }
            {
                let (left, right) = (
                    &max,
                    &usize::try_from(MAX_ROWS_PER_TABLE).map_or(usize::MAX, |value| value),
                );
                if left != right {
                    return Err(format!("left={left:?} right={right:?}").into());
                }
            }
        }
        Err(error) => return Err(error.into()),
        Ok(_) => return Err("expected the row ceiling to refuse the batch".into()),
    }
    let rows = store.scan::<CoverageRow>(Table::Coverage)?;
    if !rows.is_empty() {
        return Err(format!("expected empty coverage: {rows:?}").into());
    }
    {
        let (left, right) = (&rows_held(&store, Table::Coverage)?, &0);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    Ok(())
}
