use super::*;
#[test]
fn changed_content_is_conflict_not_overwrite() -> TestResult {
    let (_dir, store) = store()?;
    let original = input()?;
    store.put_input_record(&original)?;
    check!(matches!(
        store.put_input_record(&InputRecord::new(&id('a'), 200, &chunks())?),
        Err(StoreError::RecordConflict { kind: "input", .. })
    ));
    check!(eq; store.get_input_record(&id('a'))?, Some(original));
    Ok(())
}
#[test]
fn changed_inventory_conflicts_even_when_count_matches() -> TestResult {
    let (_dir, store) = store()?;
    store.put_input_record(&input()?)?;
    check!(matches!(
        store.put_input_record(&InputRecord::new(&id('a'), 100, &[id('f')])?),
        Err(StoreError::RecordConflict { kind: "input", .. })
    ));
    check!(eq; store.get_input_record(&id('a'))?.ok_or("missing")?.chunks(), chunks());
    Ok(())
}
#[test]
fn digest_size_limits() -> TestResult {
    check!(matches!(
        InputRecord::new(&"d".repeat(65), 100, &[]),
        Err(StoreError::Refused { .. })
    ));
    check!(eq; InputRecord::new(&id('d'), 100, &[])?.digest().len(), 64);
    Ok(())
}
#[test]
fn inventory_profile_ceiling() -> TestResult {
    check!(matches!(
        InputRecord::new(&id('a'), 100, &vec![id('e'); 4097]),
        Err(StoreError::RowTooLarge { .. })
    ));
    Ok(())
}
#[test]
fn record_kinds_do_not_collide() -> TestResult {
    let (_dir, store) = store()?;
    store.put_input_record(&input()?)?;
    store.put_build_record(&BuildRecord::new(&id('a'), &id('b'))?)?;
    store.put_unit_record(&UnitRecord::new(&id('a'), &id('b'), &id('c'))?)?;
    store.put_publish_record(&PublishRecord::new(&id('a'), &id('d'))?)?;
    check!(eq; store.get_input_record(&id('a'))?, Some(input()?));
    check!(eq; store.get_build_record(&id('a'))?, Some(BuildRecord::new(&id('a'), &id('b'))?));
    check!(eq; store.get_unit_record(&id('a'))?, Some(UnitRecord::new(&id('a'), &id('b'), &id('c'))?));
    check!(eq; store.get_publish_record(&id('a'))?, Some(PublishRecord::new(&id('a'), &id('d'))?));
    Ok(())
}
#[test]
fn repeated_put_preserves_progress() -> TestResult {
    let (_dir, store) = store()?;
    let original = input()?;
    frozen(&store)?;
    store.put_input_record(&original)?;
    check!(eq; store.get_input_record(&id('a'))?.ok_or("missing")?.revision(), 4);
    Ok(())
}
