use super::*;

enum Attempt<'a> {
    Record(&'a CanonicalSchool),
    Fail,
}

impl serde::Serialize for Attempt<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Record(record) => record.serialize(serializer),
            Self::Fail => Err(serde::ser::Error::custom("injected encode failure")),
        }
    }
}

#[test]
fn a_store_batch_encode_failure_commits_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let kept = school("Kept");
    store.append(Table::Schools, &kept)?;
    let before_seq = sequence_pointer(&store, Table::Schools)?;
    let before_rows = rows_held(&store, Table::Schools)?;
    let mut batch = store.write_batch();
    batch.journal_done("unit", "fail-atomic", &serde_json::json!({"rows": 3}))?;
    let first = school("First Staged");
    let second = school("Second Staged");
    let refused = batch.append_many(
        Table::Schools,
        &[
            Attempt::Record(&first),
            Attempt::Record(&second),
            Attempt::Fail,
        ],
    );
    if !matches!(refused, Err(StoreError::Json { .. })) {
        return Err(format!("expected encode refusal: {refused:?}").into());
    }
    drop(batch);
    drop(store);
    let reopened = Store::open(dir.path())?;
    {
        let (left, right) = (&sequence_pointer(&reopened, Table::Schools)?, &before_seq);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&rows_held(&reopened, Table::Schools)?, &before_rows);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    let journal = reopened.journal_keys("unit")?;
    if !journal.is_empty() {
        return Err(format!("expected empty journal: {journal:?}").into());
    }
    {
        let (left, right) = (
            &reopened.scan::<CanonicalSchool>(Table::Schools)?,
            &vec![kept],
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    Ok(())
}

#[test]
fn a_page_of_work_commits_rows_across_tables_and_the_journal_together() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let meet = CanonicalMeet::new(
        None,
        "Batch Invitational",
        "2026-06-01",
        CompetitionLevel::Invitational,
    );
    let mut batch = store.write_batch();
    batch.append_many(Table::Schools, std::slice::from_ref(&school("Batch High")))?;
    batch.append_many(Table::Meets, std::slice::from_ref(&meet))?;
    batch.append_many(Table::Schools, &[school("Second High")])?;
    batch.journal_done("unit", "batch-1", &serde_json::json!({ "rows": 3 }))?;
    if batch.is_empty() {
        return Err("three rows and one entry are a page".into());
    }
    batch.commit()?;
    {
        let (left, right) = (&rows_held(&store, Table::Schools)?, &2);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&rows_held(&store, Table::Meets)?, &1);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    let journal = store.journal_keys("unit")?;
    if !journal.contains("batch-1") {
        return Err(format!("the entry the page named is in the journal: {journal:?}").into());
    }
    drop(store);
    let reopened = Store::open(dir.path())?;
    {
        let (left, right) = (&reopened.scan::<CanonicalSchool>(Table::Schools)?.len(), &2);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&reopened.scan::<CanonicalMeet>(Table::Meets)?.len(), &1);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    let journal = reopened.journal_keys("unit")?;
    if !journal.contains("batch-1") {
        return Err(format!("missing committed entry: {journal:?}").into());
    }
    Ok(())
}

#[test]
fn a_refused_entry_leaves_the_page_unwritten_and_every_counter_where_it_was() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let before = sequence_pointer(&store, Table::Schools)?;
    let mut batch = store.write_batch();
    batch.append_many(Table::Schools, &[school("Never Landed")])?;
    let refused = batch.journal_done("unit", &"k".repeat(MAX_JOURNAL_KEY_BYTES + 1), &"x");
    if refused.is_ok() {
        return Err(format!("an entry with a key past its ceiling is refused: {refused:?}").into());
    }
    drop(batch);
    {
        let (left, right) = (&sequence_pointer(&store, Table::Schools)?, &before);
        if left != right {
            return Err(format!("no reservation moved — left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&rows_held(&store, Table::Schools)?, &0);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    let journal = store.journal_keys("unit")?;
    if !journal.is_empty() {
        return Err(format!("expected empty journal: {journal:?}").into());
    }
    Ok(())
}
