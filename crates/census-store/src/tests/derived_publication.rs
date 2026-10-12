use super::*;

fn publish(
    store: &Store,
    rows: &[DerivedRow],
    operation: &str,
    digest: &str,
) -> TestResult<Publication> {
    let mut stage = store.stage_derived()?;
    stage_required_tables(&mut stage, &[Table::ReviewCases])?;
    stage.replace_many(Table::ReviewCases, rows)?;
    Ok(stage.publish(operation, digest)?)
}

fn cases(count: u32) -> Vec<DerivedRow> {
    (0..count)
        .map(|index| derived(&format!("case-{index}"), 1))
        .collect()
}

#[test]
fn a_staged_generation_is_invisible_until_the_pointer_moves() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    publish(&store, &cases(5), "publish:index", "digest-one")?;
    check!(eq; store.derived_generation(), 1);
    check!(eq; store.scan::<DerivedRow>(Table::ReviewCases)?.len(), 5);
    let mut pending = store.stage_derived()?;
    stage_required_tables(&mut pending, &[Table::ReviewCases])?;
    pending.replace_many(Table::ReviewCases, &[derived("case-0", 2)])?;
    check!(eq; store.derived_generation(), 1, "staging must not move the pointer");
    check!(
        eq;
        store.scan::<DerivedRow>(Table::ReviewCases)?.len(),
        5,
        "a staged generation must stay out of reads until it is published"
    );
    pending.publish("publish:next", "digest-two")?;
    check!(eq; store.derived_generation(), 2);
    check!(eq; generation_rows(&store, Table::ReviewCases, 2)?, 1);
    check!(
        eq;
        generation_rows(&store, Table::ReviewCases, 1)?,
        0,
        "publication reclaims the generation it supersedes"
    );
    let scanned: Vec<DerivedRow> = store.scan(Table::ReviewCases)?;
    check!(eq; scanned.len(), 1);
    let Some(first) = scanned.first() else {
        return Err("the published generation holds the case it staged".into());
    };
    check!(eq; first.note, 2);
    Ok(())
}

#[test]
fn a_repeated_pass_publishes_nothing_and_leaves_its_generation_for_reclaim() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let rows = cases(3);
    publish(&store, &rows, "publish:index", "digest-one")?;
    let repeated = publish(&store, &rows, "publish:index", "digest-one")?;
    check!(
        matches!(repeated.application, Application::Repeated(_)),
        "the same operation and digest must replay its receipt rather than stage anything"
    );
    check!(eq; store.derived_generation(), 1, "a repeat must not move the pointer");
    check!(eq; store.receipt_count()?, 1, "a repeat must not record a second receipt");
    let reclaimed = store.reclaim_derived_generations(u64::MAX)?;
    check!(reclaimed.complete);
    check!(
        eq;
        generation_rows(&store, Table::ReviewCases, 2)?,
        0,
        "the abandoned staged generation must be reclaimed"
    );
    check!(eq; generation_rows(&store, Table::ReviewCases, 1)?, 3);
    Ok(())
}

#[test]
fn a_promotion_that_omits_a_required_table_is_refused_by_name() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut stage = store.stage_derived()?;
    stage_required_tables(
        &mut stage,
        &[
            Table::Coverage,
            Table::SourceIdentities,
            Table::Conflicts,
            Table::ReviewCases,
            Table::Snapshots,
        ],
    )?;
    stage.replace_many(Table::Coverage, &[derived("wi", 1)])?;
    stage.replace_many(Table::SourceIdentities, &[derived("source-identity-1", 1)])?;
    stage.replace_many(Table::Conflicts, &[derived("conflict-1", 1)])?;
    stage.replace_many(Table::ReviewCases, &[derived("case-1", 1)])?;
    let outcome = stage.publish("publish:index", "digest-one");
    let detail = match outcome {
        Err(StoreError::PublicationRefused { detail }) => detail,
        other => {
            return Err(
                format!("a manifest missing a required table must refuse: {other:?}").into(),
            )
        }
    };
    check!(
        detail.contains("snapshots"),
        "the refusal names the omitted table: {detail}"
    );
    check!(
        !detail.contains("coverage") && !detail.contains("source_identities"),
        "the refusal names only the omitted table: {detail}"
    );
    check!(eq; store.derived_generation(), 0, "a refused promotion must not move the pointer");
    check!(
        store.receipt("publish:index")?.is_none(),
        "a refused promotion must write no receipt"
    );
    check!(
        store.integrity()?.ok,
        "a refused promotion must leave a sound store"
    );
    let mut complete = store.stage_derived()?;
    stage_required_tables(&mut complete, &[Table::Coverage])?;
    complete.replace_many(Table::Coverage, &[derived("wi", 1)])?;
    let publication = complete.publish("publish:index", "digest-two")?;
    check!(
        publication.application.written(),
        "a complete manifest promotes"
    );
    check!(eq; store.derived_generation(), publication.generation);
    check!(
        publication.generation > 1,
        "the refused reservation is not reused"
    );
    check!(store.integrity()?.ok, "the promoted generation is sound");
    Ok(())
}

#[test]
fn a_generation_staged_against_moved_evidence_is_refused() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut pending = store.stage_derived()?;
    stage_required_tables(&mut pending, &[Table::ReviewCases])?;
    pending.replace_many(Table::ReviewCases, &cases(2))?;
    store.append_many(Table::Schools, &[school("Abbotsford")])?;
    let outcome = pending.publish("publish:index", "digest-one");
    check!(
        matches!(outcome, Err(StoreError::PublicationRefused { .. })),
        "a generation staged against moved inputs must be refused: {outcome:?}"
    );
    check!(eq; store.derived_generation(), 0, "a refused publication must not move the pointer");
    check!(
        eq;
        store.scan::<CanonicalSchool>(Table::Schools)?.len(),
        1,
        "the appearance that moved the inputs must survive the refusal"
    );
    check!(
        eq;
        generation_rows(&store, Table::ReviewCases, 1)?,
        2,
        "the refused generation stays staged and unpublished"
    );
    let reclaimed = store.reclaim_derived_generations(u64::MAX)?;
    check!(reclaimed.complete);
    check!(eq; generation_rows(&store, Table::ReviewCases, 1)?, 0);
    Ok(())
}

#[test]
fn omitting_each_required_table_preserves_the_previous_complete_generation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let mut initial = store.stage_derived()?;
    let required: Vec<Table> = Table::ALL
        .into_iter()
        .filter(|table| table.generation_partitioned())
        .collect();
    for table in &required {
        initial.replace_many(*table, &[derived(table.file(), 7)])?;
    }
    let retained = initial.publish("publish:complete", "complete")?.generation;
    for omitted in &required {
        let mut stage = store.stage_derived()?;
        for table in &required {
            if table != omitted {
                stage.carry_forward(*table)?;
            }
        }
        let staged = stage.generation();
        let operation = format!("publish:omit:{}", omitted.file());
        let detail = match stage.publish(&operation, "missing") {
            Err(StoreError::PublicationRefused { detail }) => detail,
            other => return Err(format!("omitted {omitted:?}: {other:?}").into()),
        };
        check!(
            eq;
            detail,
            format!(
                "the manifest for generation {staged} omits required derived table(s) {}: a promoted \
                 generation must declare every generation-partitioned table it makes current",
                omitted.file()
            )
        );
        check!(eq; store.derived_generation(), retained);
        check!(store.receipt(&operation)?.is_none());
        for table in &required {
            let rows = store.scan::<DerivedRow>(*table)?;
            check!(eq; rows.len(), 1);
            let row = rows.first().ok_or("previous generation lost its row")?;
            check!(eq; row.id.as_str(), table.file());
            check!(eq; row.note, 7);
        }
    }
    Ok(())
}
