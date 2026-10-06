use super::*;
use census_domain::model::CanonicalSchool;

const TABLES: [Table; 6] = [
    Table::Meets,
    Table::Events,
    Table::Teams,
    Table::Athletes,
    Table::Performances,
    Table::SourceObservations,
];

fn state(store: &Store) -> TestResult<(u64, String, Vec<census_store::TableWalk>)> {
    let snapshot = store.snapshot();
    Ok((
        snapshot.sequence(),
        snapshot.tables_digest(&TABLES)?,
        TABLES
            .into_iter()
            .map(|table| Ok(store.walk_table(table)?))
            .collect::<TestResult<_>>()?,
    ))
}

fn seed_schools(store: &Store, schools: &[CanonicalSchool]) -> TestResult {
    for school in schools {
        store.append(census_store::Table::Schools, school)?;
    }
    Ok(())
}

fn partial(store: &Store) -> TestResult {
    let receipts = store.journal_payloads(super::super::super::RESULT_SET_PHASE)?;
    check!(receipts.iter().any(|row| row["disposition"] == "partial"
        && row["meet"] == "725218"
        && row["rsid"] == "1266814"));
    check!(!receipts
        .iter()
        .any(|row| row["disposition"] == "projection_applied"));
    Ok(())
}

async fn collect_report(
    store: &Store,
    fetcher: &Fetcher,
    reference: &ResultSetRef,
    errors: u64,
) -> TestResult {
    let report =
        crate::milesplit::collect_result_sets(&context(store, fetcher)?, &options(reference))
            .await?;
    check!(eq; (report.rows, report.errors), (3, errors));
    Ok(())
}

fn resolved(store: &Store) -> TestResult {
    for (table, rows) in [
        (Table::Meets, 1),
        (Table::Events, 3),
        (Table::Teams, 2),
        (Table::Athletes, 2),
        (Table::Performances, 3),
        (Table::SourceObservations, 3),
    ] {
        check!(eq;
            store.walk_table(table)?.rows,
            rows,
            "{table:?}"
        );
    }
    let marks: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
    check!(marks
        .iter()
        .any(|row| row.source_key == "milesplit_result:201782806"));
    check!(store
        .journal_payloads(super::super::super::RESULT_SET_PHASE)?
        .iter()
        .any(|row| row["disposition"] == "projection_applied"
            && row["meet"] == "725218"
            && row["rsid"] == "1266814"
            && row["projected_rows"] == 3));
    Ok(())
}

#[test]
fn partial_cohort_projection_reopens_without_duplicating_retained_or_projected_rows() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher, reference) = setup()?;
            let mut given: serde_json::Value = serde_json::from_slice(TROY)?;
            given["data"][0]["gradYear"] = serde_json::Value::Null;
            seed_owned(&fetcher, &reference, &serde_json::to_vec(&given)?)?;
            seed_metadata(&fetcher, &reference)?;
            collect_report(&store, &fetcher, &reference, 1).await?;
            check!(eq;
                store
                    .walk_table(Table::Performances)
                    ?
                    .rows,
                2
            );
            check!(eq;
                store
                    .walk_table(Table::SourceObservations)
                    ?
                    .rows,
                3
            );
            partial(&store)?;
            let before = state(&store)?;
            let receipts = store.journal_payloads(super::super::super::RESULT_SET_PHASE)?;
            store.flush()?;
            drop(store);
            let store = Store::open(dir.path().join("store"))?;
            collect_report(&store, &fetcher, &reference, 1).await?;
            check!(eq; state(&store)?, before);
            check!(eq;
                store
                    .journal_payloads(super::super::super::RESULT_SET_PHASE)
                    ?,
                receipts
            );
            partial(&store)?;
            Ok(())
        })
}

#[test]
fn a_genuine_new_school_binding_resumes_partial_projection_without_reappending_prior_facts(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher, reference) = setup_bare()?;
            seed_owned(&fetcher, &reference, TROY)?;
            seed_metadata(&fetcher, &reference)?;
            collect_report(&store, &fetcher, &reference, 1).await?;
            check!(eq;
                store
                    .walk_table(Table::Performances)
                    ?
                    .rows,
                0
            );
            partial(&store)?;
            let before = state(&store)?;
            let prior = store.journal_payloads(super::super::super::RESULT_SET_PHASE)?;
            store.flush()?;
            drop(store);
            let store = Store::open(dir.path().join("store"))?;
            collect_report(&store, &fetcher, &reference, 1).await?;
            check!(eq; state(&store)?, before);
            seed_schools(
                &store,
                &[
                    school("Spann provider school", "38332"),
                    school("Charles", "4912"),
                ],
            )?;
            collect_report(&store, &fetcher, &reference, 0).await?;
            resolved(&store)?;
            let after = store.journal_payloads(super::super::super::RESULT_SET_PHASE)?;
            check!(prior.iter().all(|receipt| after.contains(receipt)));
            let completed = state(&store)?;
            collect_report(&store, &fetcher, &reference, 0).await?;
            check!(eq; state(&store)?, completed);
            Ok(())
        })
}

#[test]
fn a_second_exact_school_claim_keeps_the_binding_ambiguous_and_preserves_prior_facts() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            seed_owned(&fetcher, &reference, TROY)?;
            seed_metadata(&fetcher, &reference)?;
            collect_report(&store, &fetcher, &reference, 0).await?;
            let prior_receipts = store.journal_payloads(super::super::super::RESULT_SET_PHASE)?;
            let prior_marks: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            let original = prior_marks
                .iter()
                .find(|row| row.source_key == "milesplit_result:201782263")
                .ok_or("original result")?;
            let changed_school = school("New exact canonical owner", "38332");
            seed_schools(&store, std::slice::from_ref(&changed_school))?;
            let report = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; report.rows, 3);
            check!(eq; report.errors, 1);
            check!(eq;
                report.unresolved,
                Some(crate::UnresolvedCounters { rows: 2, labels: 1 })
            );
            let marks: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(marks.contains(original));
            check!(eq; marks.len(), 3);
            let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
            check!(eq; athletes.len(), 2);
            check!(!athletes.iter().any(|row| row.school == changed_school.id));
            check!(eq;
                store
                    .walk_table(Table::SourceObservations)
                    ?
                    .rows,
                3
            );
            let after_receipts = store.journal_payloads(super::super::super::RESULT_SET_PHASE)?;
            check!(prior_receipts
                .iter()
                .all(|receipt| after_receipts.contains(receipt)));
            let applied: Vec<_> = after_receipts
                .iter()
                .filter(|row| row["disposition"] == "projection_applied")
                .collect();
            check!(eq; applied.len(), 1);
            let before_replay = state(&store)?;
            let report = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; report.errors, 1);
            check!(eq; state(&store)?, before_replay);
            check!(eq;
                store
                    .journal_payloads(super::super::super::RESULT_SET_PHASE)
                    ?,
                after_receipts
            );
            Ok(())
        })
}
