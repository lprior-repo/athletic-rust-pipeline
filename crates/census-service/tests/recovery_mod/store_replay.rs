use super::material::{ks_pass, pending_units, process_unit, process_units, store_seeded_with_ks};
use super::{
    count_of, journal_keys, note, open_store, school_ids, table_counts, CanonicalCoach,
    CanonicalSchool, Table, OBSERVED_ON, UNIT_PHASE,
};

#[test]
fn store_reopen_after_a_writer_stops_mid_batch_resumes_at_the_first_unjournaled_unit(
) -> super::TestResult {
    const SCENARIO: &str = "store-resume";
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let units = ["unit-1", "unit-2", "unit-3", "unit-4"];

    let (journal_before, counts_before, merged_before, consolidated_before) = {
        let store = open_store(&root)?;
        process_units(&store, &units[..2], OBSERVED_ON)?;
        let journal = journal_keys(&store, UNIT_PHASE)?;
        let counts = table_counts(&store)?;
        let merged = store.scan::<CanonicalSchool>(Table::Schools)?;
        let consolidated = store
            .consolidate::<CanonicalSchool>(Table::Schools, &root.join("out/schools.jsonl"))?;
        (journal, counts, merged.len(), consolidated)
    };
    note(
        SCENARIO,
        format!(
            "before restart: journal={:?} tables={counts_before:?} merged_entities={merged_before} \
             snapshot_rows={}",
            journal_before, consolidated_before.rows
        ),
    );
    check!(eq; journal_before.len(), 2, "two units reached the journal");
    check!(eq; count_of(&counts_before, Table::Schools), 2);
    check!(eq; merged_before, 2);

    let store = open_store(&root)?;
    let journal_after = journal_keys(&store, UNIT_PHASE)?;
    let counts_after = table_counts(&store)?;
    note(
        SCENARIO,
        format!("after restart: journal={journal_after:?} tables={counts_after:?}"),
    );
    check!(eq; journal_after, journal_before,
    "the resume ledger is the durable record of finished work and must survive the restart");
    check!(eq; counts_after, counts_before,
    "reopening replays exactly what reached the journal; counters are unchanged");

    let pending = pending_units(&store, &units)?;
    note(SCENARIO, format!("resume set: pending={pending:?}"));
    check!(eq; pending,
    vec!["unit-3", "unit-4"],
    "the resumed worker starts at the first unit the journal does not claim");
    process_units(&store, &pending, OBSERVED_ON)?;
    let finished_journal = journal_keys(&store, UNIT_PHASE)?;
    let finished_counts = table_counts(&store)?;
    note(
        SCENARIO,
        format!(
            "after the resumed units: journal={} schools={} observations={}",
            finished_journal.len(),
            count_of(&finished_counts, Table::Schools),
            store.stats()?.observations
        ),
    );
    check!(eq; finished_journal.len(), 4);
    check!(eq; count_of(&finished_counts, Table::Schools), 4);

    process_unit(&store, "unit-1", "2026-09-21")?;
    let after_duplicate = store.stats()?.observations;
    let merged = store.scan::<CanonicalSchool>(Table::Schools)?;
    let unit_1 = merged
        .iter()
        .find(|school| school.name == "Recovery Unit unit-1")
        .ok_or("unit-1 not merged")?;
    let dates: Vec<&str> = unit_1
        .evidence
        .iter()
        .map(|evidence| evidence.observed_on.as_str())
        .collect();
    note(
        SCENARIO,
        format!(
            "duplicate observation: observations={after_duplicate} merged_entities={} \
             evidence_on_unit_1={dates:?}",
            merged.len()
        ),
    );
    check!(eq; after_duplicate, 5,
    "the reopened store continues the sequence instead of overwriting the earlier row");
    check!(eq; merged.len(),
    4,
    "a duplicate observation reconciles into the same entity: no double count");
    check!(eq; unit_1.evidence.len(),
    2,
    "both observations stay as evidence on the one merged entity");
    Ok(())
}

#[test]
fn adapter_restart_reuses_finished_units_without_refetch_or_duplicate_rows() -> super::TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let dir = tempfile::tempdir()?;
        let control_root = dir.path().join("control");
        let (control_units, control_schools, control_coaches, control_counts, control_receipts) = {
            let store = store_seeded_with_ks(&control_root)?;
            let report = ks_pass(&store, None).await?;
            check!(eq; report.requests, 0);
            check!(eq; report.rows, 5);
            check!(eq; report.disposition, census_crawl::CollectionDisposition::Partial);
            check!(eq; report.unfinished,
                vec!["https://kshsaa-api.kshsaa.org/directory/search/name/".to_string()]);
            (school_ids(&store)?, store.scan::<CanonicalSchool>(Table::Schools)?,
                store.scan::<CanonicalCoach>(Table::Coaches)?, table_counts(&store)?,
                store.receipt_count()?)
        };
        check!(eq; control_units.len(), 5);
        check!(eq; control_coaches.len(), 5);
        let restart_root = dir.path().join("restart");
        let first_units = {
            let store = store_seeded_with_ks(&restart_root)?;
            let report = ks_pass(&store, Some(2)).await?;
            check!(eq; report.rows, 2);
            check!(eq; report.requests, 0);
            check!(eq; school_ids(&store)?.len(), 2);
            school_ids(&store)?
        };
        let store = open_store(&restart_root)?;
        let resumed = ks_pass(&store, None).await?;
        check!(eq; resumed.rows, 3);
        check!(eq; resumed.requests, 0);
        check!(eq; school_ids(&store)?, control_units);
        check!(first_units.is_subset(&school_ids(&store)?));
        check!(eq; store.scan::<CanonicalSchool>(Table::Schools)?, control_schools);
        check!(eq; store.scan::<CanonicalCoach>(Table::Coaches)?, control_coaches);
        check!(eq; table_counts(&store)?, control_counts);
        check!(eq; store.receipt_count()?, control_receipts);
        let replay = ks_pass(&store, None).await?;
        check!(eq; replay.rows, 0);
        check!(eq; table_counts(&store)?, control_counts);
        check!(eq; store.receipt_count()?, control_receipts);
        Ok(())
    })
}
