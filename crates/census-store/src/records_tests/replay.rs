use super::*;
#[test]
fn replay_reuses_complete_units() -> TestResult {
    let (_dir, store) = store()?;
    verified(&store)?;
    check!(store.unit_replay_complete(&id('c'), &[id('a'), id('b')])?);
    Ok(())
}
#[test]
fn replay_refuses_incomplete_units() -> TestResult {
    let (_dir, store) = store()?;
    store.put_unit_record(&UnitRecord::new(&id('c'), &id('a'), &id('b'))?)?;
    check!(!store.unit_replay_complete(&id('c'), &[])?);
    Ok(())
}
#[test]
fn inventory_reconciliation_finds_missing() -> TestResult {
    let (_dir, store) = store()?;
    store.put_unit_record(&UnitRecord::new(&id('c'), &id('a'), &id('b'))?)?;
    check!(eq; store.inventory_reconcile(&[id('c'), id('f')])?, vec![id('c'), id('f')]);
    Ok(())
}
#[test]
fn inventory_reconciliation_all_present() -> TestResult {
    let (_dir, store) = store()?;
    verified(&store)?;
    check!(eq; store.inventory_reconcile(&[id('c')])?, Vec::<String>::new());
    Ok(())
}
#[test]
fn persisted_transition_survives_reopen() -> TestResult {
    let (dir, store) = store()?;
    verified(&store)?;
    let input = store.get_input_record(&id('a'))?;
    let build = store.get_build_record(&id('b'))?;
    let unit = store.get_unit_record(&id('c'))?;
    drop(store);
    let reopened = Store::open(dir.path())?;
    check!(eq; reopened.get_input_record(&id('a'))?, input);
    check!(eq; reopened.get_build_record(&id('b'))?, build);
    check!(eq; reopened.get_unit_record(&id('c'))?, unit);
    check!(reopened.unit_replay_complete(&id('c'), &[])?);
    Ok(())
}
