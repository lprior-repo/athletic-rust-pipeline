use super::*;
use serde::{Deserialize, Serialize};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Row {
    id: String,
    note: String,
}

impl Entity for Row {
    fn entity_id(&self) -> &str {
        &self.id
    }
    fn merge(&mut self, other: Self) {
        *self = other;
    }
}

fn row(id: &str, note: &str) -> Row {
    Row {
        id: id.to_string(),
        note: note.to_string(),
    }
}

fn rows_of(store: &Store, table: Table) -> TestResult<Vec<Row>> {
    let mut rows = store.scan::<Row>(table)?;
    rows.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(rows)
}

fn count(store: &Store, table: Table) -> TestResult<u64> {
    Ok(store
        .stats()?
        .tables
        .into_iter()
        .find(|(name, _)| name == table.file())
        .map(|(_, count)| count)
        .ok_or("table row count")?)
}

fn derive<'s>(store: &'s Store, verdicts: &[Row], cases: &[Row]) -> TestResult<StoreBatch<'s>> {
    let mut batch = store.write_batch();
    batch.replace_many(Table::IdentityVerdicts, verdicts)?;
    batch.replace_many(Table::ReviewCases, cases)?;
    Ok(batch)
}

#[test]
fn one_derivation_writes_its_tables_in_one_commit() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let verdicts = vec![row("verdict-1", "decided"), row("verdict-2", "refused")];
    let cases = vec![row("case-1", "closed"), row("case-2", "retained")];
    let applied =
        derive(&store, &verdicts, &cases)?.commit_once("review:2026-09-25:7", "digest-7")?;
    check!(
        applied.written(),
        "the first application wrote the derivation"
    );
    check!(eq; rows_of(&store, Table::IdentityVerdicts)?, verdicts, "the verdicts are readable");
    check!(eq; rows_of(&store, Table::ReviewCases)?, cases, "and the cases' new state is readable beside them");
    check!(eq; count(&store, Table::IdentityVerdicts)?, 2);
    check!(eq; count(&store, Table::ReviewCases)?, 2);
    check!(eq; store.receipt_count()?, 1, "one derivation, one receipt");
    Ok(())
}

#[test]
fn a_replayed_derivation_writes_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let verdicts = vec![row("verdict-1", "decided")];
    let cases = vec![row("case-1", "closed")];
    derive(&store, &verdicts, &cases)?.commit_once("review:2026-09-25:7", "digest-7")?;
    let replay =
        derive(&store, &verdicts, &cases)?.commit_once("review:2026-09-25:7", "digest-7")?;
    check!(
        replay.repeated(),
        "the second application is the first's replay"
    );
    check!(eq; rows_of(&store, Table::IdentityVerdicts)?, verdicts);
    check!(eq; rows_of(&store, Table::ReviewCases)?, cases);
    check!(eq; count(&store, Table::IdentityVerdicts)?, 1);
    check!(eq; count(&store, Table::ReviewCases)?, 1);
    check!(eq; store.receipt_count()?, 1);
    Ok(())
}

#[test]
fn a_derivation_that_never_commits_leaves_both_tables_as_they_were() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    store.replace_many(Table::ReviewCases, &[row("case-1", "open")])?;
    drop(derive(
        &store,
        &[row("verdict-1", "decided")],
        &[row("case-1", "closed")],
    )?);
    check!(
        rows_of(&store, Table::IdentityVerdicts)?.is_empty(),
        "the dropped batch left no verdict"
    );
    check!(eq; rows_of(&store, Table::ReviewCases)?, vec![row("case-1", "open")], "and the case still reads as the last commit left it");
    Ok(())
}

#[test]
fn a_snapshot_table_replaced_by_nothing_comes_out_empty() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let held = vec![row("wi", "covered"), row("mn", "covered")];
    {
        let mut batch = store.write_batch();
        batch.replace_many(Table::Coverage, &held)?;
        batch.commit()?;
    }
    check!(eq; count(&store, Table::Coverage)?, 2);
    let mut empty = store.write_batch();
    empty.replace_many(Table::Coverage, &Vec::<Row>::new())?;
    empty.commit()?;
    check!(
        rows_of(&store, Table::Coverage)?.is_empty(),
        "a snapshot derivation names the table's whole content, so naming none empties it"
    );
    check!(eq; count(&store, Table::Coverage)?, 0);
    Ok(())
}

#[test]
fn one_table_cannot_be_appended_and_replaced_in_one_batch() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut batch = store.write_batch();
    batch.append_many(Table::ReviewCases, &[row("case-1", "appended")])?;
    let refused = batch.replace_many(Table::ReviewCases, &[row("case-1", "replaced")]);
    check!(
        refused.is_err(),
        "an append and a replacement disagree about what the table holds"
    );
    drop(batch);
    check!(
        store.scan::<Row>(Table::ReviewCases)?.is_empty(),
        "and a refused call writes nothing"
    );
    Ok(())
}

#[test]
fn a_refused_append_after_a_replacement_leaves_the_batch_committable() -> TestResult {
    let dir = tempfile::tempdir()?;
    let applied = {
        let store = Store::open(dir.path())?;
        let mut batch = store.write_batch();
        batch.replace_many(Table::ReviewCases, &[row("case-1", "replaced")])?;
        check!(matches!(
            batch.append_many(Table::ReviewCases, &[row("case-2", "appended")]),
            Err(StoreError::Invariant { .. })
        ));
        let applied = batch.commit_once("review:2026-10-05:1", "digest-1")?;
        check!(applied.written(), "the replacement still commits alone");
        check!(eq; rows_of(&store, Table::ReviewCases)?, vec![row("case-1", "replaced")]);
        check!(eq; store.walk_table(Table::ReviewCases)?.rows, 1);
        check!(store.integrity()?.ok);
        applied.receipt().clone()
    };
    let reopened = Store::open(dir.path())?;
    check!(eq; reopened.walk_table(Table::ReviewCases)?.rows, 1);
    check!(eq; reopened.receipt("review:2026-10-05:1")?, Some(applied));
    check!(reopened.integrity()?.ok);
    Ok(())
}

const OBSERVATION_LOGS: [Table; 9] = [
    Table::Schools,
    Table::Teams,
    Table::Coaches,
    Table::Athletes,
    Table::Meets,
    Table::Events,
    Table::Performances,
    Table::SourceMeets,
    Table::SourceObservations,
];

#[test]
fn observation_history_refuses_replacement_before_and_after_an_append() -> TestResult {
    let dir = tempfile::tempdir()?;
    {
        let store = Store::open(dir.path())?;
        for table in OBSERVATION_LOGS {
            let replacement = [row("row-1", "replacement")];
            check!(matches!(
                store.replace_many(table, &replacement),
                Err(StoreError::ObservationReplacement { .. })
            ));
            check!(matches!(
                store.replace_many::<Row>(table, &[]),
                Err(StoreError::ObservationReplacement { .. })
            ));
            check!(eq; store.walk_table(table)?.rows, 0);
            store.append(table, &row("row-1", "first"))?;
            check!(matches!(
                store.replace_many(table, &replacement),
                Err(StoreError::ObservationReplacement { .. })
            ));
            let mut batch = store.write_batch();
            check!(matches!(
                batch.replace_many(table, &replacement),
                Err(StoreError::ObservationReplacement { .. })
            ));
            check!(matches!(
                batch.replace_many::<Row>(table, &[]),
                Err(StoreError::ObservationReplacement { .. })
            ));
            batch.commit()?;
            check!(eq; rows_of(&store, table)?, vec![row("row-1", "first")]);
            store.append(table, &row("row-1", "second"))?;
            check!(eq; store.walk_table(table)?.rows, 2);
        }
        check!(store.integrity()?.ok);
    }
    let reopened = Store::open(dir.path())?;
    for table in OBSERVATION_LOGS {
        check!(eq; reopened.walk_table(table)?.rows, 2);
        check!(eq; rows_of(&reopened, table)?, vec![row("row-1", "second")]);
    }
    check!(reopened.integrity()?.ok);
    Ok(())
}
