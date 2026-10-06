use census_domain::model::{normalize_name, CanonicalSchool};
use census_domain::UsJurisdiction;
use census_store::{Application, Entity, Reclaimed, Store, StoreError, Table};
use serde::{Deserialize, Serialize};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Row {
    id: String,
    note: u32,
}

impl Entity for Row {
    fn entity_id(&self) -> &str {
        &self.id
    }

    fn merge(&mut self, other: Self) {
        *self = other;
    }
}

fn row(id: &str, note: u32) -> Row {
    Row {
        id: id.to_string(),
        note,
    }
}

fn published(
    store: &Store,
    table: Table,
    rows: &[Row],
    operation: &str,
    digest: &str,
) -> TestResult<u64> {
    let mut stage = store.stage_derived()?;
    stage.replace_many(table, rows)?;
    Ok(stage.publish(operation, digest)?.generation)
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

fn notes(store: &Store, table: Table) -> TestResult<Vec<(String, u32)>> {
    let mut rows: Vec<(String, u32)> = store
        .scan::<Row>(table)?
        .into_iter()
        .map(|row| (row.id, row.note))
        .collect();
    rows.sort();
    Ok(rows)
}

#[test]
fn one_stage_holds_one_row_per_id_and_the_last_write_wins() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let publication = {
        let mut stage = store.stage_derived()?;
        stage.replace_many(Table::Coverage, &[row("wi", 1), row("mn", 2), row("wi", 7)])?;
        stage.publish("derive:coverage:duplicate-id", "digest-duplicate-id")?
    };
    if !matches!(publication.application, Application::Written(_)) {
        return Err(format!("the first publication writes: {publication:?}").into());
    }
    let rows = notes(&store, Table::Coverage)?;
    if rows != vec![("mn".to_string(), 2), ("wi".to_string(), 7)] {
        return Err(format!("one row per id, the last write kept: {rows:?}").into());
    }
    let walk = store.walk_table(Table::Coverage)?;
    if walk.rows != 2 || walk.repeated_ids != 0 {
        return Err(format!("a generation repeats no id: {walk:?}").into());
    }
    Ok(())
}

#[test]
fn a_published_generation_is_invisible_until_publish_and_durable_across_reopen() -> TestResult {
    let dir = tempfile::tempdir()?;
    let root = dir.path();
    let store = Store::open(root)?;
    {
        let mut stage = store.stage_derived()?;
        stage.replace_many(Table::Coverage, &[row("wi", 1), row("mn", 1)])?;
        if !store.scan::<Row>(Table::Coverage)?.is_empty() {
            return Err("a staged generation is invisible until it is published".into());
        }
        let publication = stage.publish("derive:coverage:1", "digest-1")?;
        if !publication.application.written() {
            return Err(format!("the first publication must write: {publication:?}").into());
        }
        {
            let (left, right) = (&publication.generation, &1);
            if left != right {
                return Err(format!("left={left:?} right={right:?}").into());
            }
        }
        if publication.reclaimed.rows != 0 {
            return Err(format!("nothing is stale yet: {publication:?}").into());
        }
    }
    {
        let (left, right) = (&store.derived_generation(), &1);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&store.evidence_generation(), &1);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&count(&store, Table::Coverage)?, &2);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    store.replace_many(Table::Coverage, &[row("oh", 1)])?;
    drop(store);

    let store = Store::open(root)?;
    {
        let (left, right) = (&store.derived_generation(), &1);
        if left != right {
            return Err(format!(
                "the published pointer survives a reopen — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    {
        let (left, right) = (
            &notes(&store, Table::Coverage)?,
            &vec![
                ("mn".to_string(), 1),
                ("oh".to_string(), 1),
                ("wi".to_string(), 1),
            ],
        );
        if left != right {
            return Err(format!(
                "an in-place write lands beside the published generation — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    if store.receipt("derive:coverage:1")?.is_none() {
        return Err("the publication receipt is durable".into());
    }
    if !store.integrity()?.ok {
        return Err("the store must pass its own integrity check".into());
    }
    Ok(())
}

#[test]
fn republishing_one_operation_and_digest_is_a_repeat_and_a_different_digest_is_refused(
) -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    published(
        &store,
        Table::Coverage,
        &[row("wi", 1)],
        "derive:coverage:1",
        "digest-1",
    )?;
    let repeat = {
        let mut stage = store.stage_derived()?;
        stage.replace_many(Table::Coverage, &[row("mn", 9)])?;
        stage.publish("derive:coverage:1", "digest-1")?
    };
    if !matches!(repeat.application, Application::Repeated(_)) {
        return Err(format!("a replay repeats rather than writes: {repeat:?}").into());
    }
    if repeat.reclaimed.rows == 0 {
        return Err(format!("a repeat reclaims the stage it abandoned: {repeat:?}").into());
    }
    let swept = store.reclaim_derived_generations(u64::MAX)?;
    if swept.rows != 0 || !swept.complete {
        return Err(format!("no abandoned stage survives a repeat: {swept:?}").into());
    }
    {
        let (left, right) = (&store.derived_generation(), &1);
        if left != right {
            return Err(
                format!("a repeat publishes nothing — left={left:?} right={right:?}").into(),
            );
        }
    }
    {
        let (left, right) = (
            &notes(&store, Table::Coverage)?,
            &vec![("wi".to_string(), 1)],
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    let outcome = {
        let mut stage = store.stage_derived()?;
        stage.replace_many(Table::Coverage, &[row("mn", 9)])?;
        stage.publish("derive:coverage:1", "digest-2")
    };
    if !matches!(outcome, Err(StoreError::Invariant { .. })) {
        return Err(format!("one operation cannot name two payloads: {outcome:?}").into());
    }
    {
        let (left, right) = (&store.derived_generation(), &1);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    if !store.integrity()?.ok {
        return Err("a refused publication leaves a sound store".into());
    }
    let swept = store.reclaim_derived_generations(u64::MAX)?;
    if swept.rows == 0 || !swept.complete {
        return Err(format!("a refusal leaves its stage for reclaim: {swept:?}").into());
    }
    Ok(())
}

#[test]
fn a_generation_staged_against_moved_evidence_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    published(
        &store,
        Table::Coverage,
        &[row("wi", 1)],
        "derive:coverage:1",
        "digest-1",
    )?;
    let school = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Moved Evidence School",
        normalize_name("Moved Evidence School"),
        None,
    )
    .0;
    let base_evidence = store.evidence_generation();
    let outcome = {
        let mut stage = store.stage_derived()?;
        stage.replace_many(Table::Coverage, &[row("mn", 9)])?;
        store.append(Table::Schools, &school)?;
        stage.publish("derive:coverage:2", "digest-2")
    };
    if !matches!(outcome, Err(StoreError::PublicationRefused { .. })) {
        return Err(format!("a stale input generation must refuse: {outcome:?}").into());
    }
    if store.evidence_generation() == base_evidence {
        return Err("the fixture did not move the evidence generation".into());
    }
    {
        let (left, right) = (&store.derived_generation(), &1);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (
            &notes(&store, Table::Coverage)?,
            &vec![("wi".to_string(), 1)],
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    if store.receipt("derive:coverage:2")?.is_some() {
        return Err("a refused publication writes no receipt".into());
    }
    let retried = published(
        &store,
        Table::Coverage,
        &[row("wi", 1), row("mn", 2)],
        "derive:coverage:2",
        "digest-2",
    )?;
    {
        let (left, right) = (&retried, &3);
        if left != right {
            return Err(format!(
                "a refused stage burned its reservation, so the retry lands above it: \
                 left={left:?} right={right:?}"
            )
            .into());
        }
    }
    {
        let (left, right) = (&store.derived_generation(), &3);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    Ok(())
}

#[test]
fn publishing_reclaims_the_generation_it_supersedes() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    published(
        &store,
        Table::Coverage,
        &[row("wi", 1), row("mn", 1)],
        "derive:coverage:1",
        "digest-1",
    )?;
    let second = {
        let mut stage = store.stage_derived()?;
        stage.replace_many(Table::Coverage, &[row("wi", 1)])?;
        stage.publish("derive:coverage:2", "digest-2")?
    };
    {
        let (left, right) = (
            &second.reclaimed,
            &Reclaimed {
                rows: 2,
                complete: true,
            },
        );
        if left != right {
            return Err(format!(
                "the superseded generation's rows are reclaimed — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    {
        let (left, right) = (&store.derived_generation(), &2);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (
            &notes(&store, Table::Coverage)?,
            &vec![("wi".to_string(), 1)],
        );
        if left != right {
            return Err(format!(
                "publication replaces the rows it does not stage — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    {
        let (left, right) = (&count(&store, Table::Coverage)?, &1);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    let nothing = store.reclaim_derived_generations(u64::MAX)?;
    {
        let (left, right) = (
            &nothing,
            &Reclaimed {
                rows: 0,
                complete: true,
            },
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (
            &notes(&store, Table::Coverage)?,
            &vec![("wi".to_string(), 1)],
        );
        if left != right {
            return Err(format!(
                "the current generation is never reclaimed — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    Ok(())
}

#[test]
fn reclaim_honours_its_budget_and_finishes_on_the_next_call() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let rows: Vec<Row> = (0..26_000)
        .map(|index| row(&format!("row-{index:05}"), 1))
        .collect();
    {
        let mut stage = store.stage_derived()?;
        stage.replace_many(Table::Coverage, &rows)?;
    }
    let partial = store.reclaim_derived_generations(1_000)?;
    {
        let (left, right) = (
            &partial,
            &Reclaimed {
                rows: 1_000,
                complete: false,
            },
        );
        if left != right {
            return Err(format!(
                "a bounded reclaim deletes exactly its budget — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    let rest = store.reclaim_derived_generations(u64::MAX)?;
    if !rest.complete {
        return Err(format!("the second reclaim finishes: {rest:?}").into());
    }
    let total = partial.rows.checked_add(rest.rows).ok_or("count")?;
    if total < 25_000 {
        return Err(
            format!("the abandoned stage flushed its over-budget batches: {total} rows").into(),
        );
    }
    let again = store.reclaim_derived_generations(u64::MAX)?;
    {
        let (left, right) = (&again.rows, &0);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    if !store.integrity()?.ok {
        return Err("a reclaimed store is sound".into());
    }
    Ok(())
}

#[test]
fn carry_forward_moves_the_current_generation_into_the_next() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    published(
        &store,
        Table::Coverage,
        &[row("wi", 1), row("mn", 1)],
        "derive:coverage:1",
        "digest-1",
    )?;
    let moved = {
        let mut stage = store.stage_derived()?;
        let moved = stage.carry_forward(Table::Coverage)?;
        stage.replace_many(Table::Coverage, &[row("oh", 3)])?;
        {
            let (left, right) = (
                &notes(&store, Table::Coverage)?,
                &vec![("mn".to_string(), 1), ("wi".to_string(), 1)],
            );
            if left != right {
                return Err(format!(
                    "a carried generation is invisible until it publishes — left={left:?} right={right:?}"
                )
                .into());
            }
        }
        stage.publish("derive:coverage:2", "digest-2")?;
        moved
    };
    {
        let (left, right) = (&moved, &2);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (
            &notes(&store, Table::Coverage)?,
            &vec![
                ("mn".to_string(), 1),
                ("oh".to_string(), 3),
                ("wi".to_string(), 1),
            ],
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&count(&store, Table::Coverage)?, &3);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    Ok(())
}

#[test]
fn a_generation_larger_than_one_staging_batch_publishes_whole() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let rows: Vec<Row> = (0..26_000)
        .map(|index| row(&format!("row-{index:05}"), index))
        .collect();
    let generation = published(
        &store,
        Table::Coverage,
        &rows,
        "derive:coverage:1",
        "digest-1",
    )?;
    {
        let (left, right) = (&generation, &1);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (&count(&store, Table::Coverage)?, &26_000);
        if left != right {
            return Err(format!(
                "every row of a multi-batch generation counts — left={left:?} right={right:?}"
            )
            .into());
        }
    }
    let scanned = store.scan::<Row>(Table::Coverage)?;
    {
        let (left, right) = (&scanned.len(), &26_000);
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    {
        let (left, right) = (
            &scanned.iter().map(|row| row.note).sum::<u32>(),
            &rows.iter().map(|row| row.note).sum::<u32>(),
        );
        if left != right {
            return Err(format!("left={left:?} right={right:?}").into());
        }
    }
    if !store.integrity()?.ok {
        return Err("the published generation is sound".into());
    }
    Ok(())
}

#[test]
fn staging_refuses_tables_that_are_not_generation_partitioned() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Unpartitioned School",
        normalize_name("Unpartitioned School"),
        None,
    )
    .0;
    let mut stage = store.stage_derived()?;
    for outcome in [
        stage.replace_many(Table::IdentityVerdicts, &[row("verdict:1", 1)]),
        stage.replace_many(Table::Schools, std::slice::from_ref(&school)),
    ] {
        if !matches!(outcome, Err(StoreError::NotGenerationTable { .. })) {
            return Err(format!("only generation tables are staged: {outcome:?}").into());
        }
    }
    if !matches!(
        stage.carry_forward(Table::Schools),
        Err(StoreError::NotGenerationTable { .. })
    ) {
        return Err("carry-forward refuses a table the store does not partition".into());
    }
    drop(stage);
    if !store.scan::<Row>(Table::Coverage)?.is_empty() {
        return Err("a refused stage writes nothing".into());
    }
    Ok(())
}
