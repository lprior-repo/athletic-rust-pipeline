use super::material::{ks_pass, pending_units, process_unit, process_units, store_seeded_with_ks};
use super::{
    count_of, digest_of, journal_keys, note, open_store, table_counts, CanonicalCoach,
    CanonicalSchool, Table, KS_PHASE, OBSERVED_ON, UNIT_PHASE,
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
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
    const SCENARIO: &str = "adapter-resume";
    let dir = tempfile::tempdir()?;

    let control_root = dir.path().join("control");
    let (control_journal, control_schools, control_coaches, control_report, control_counts) = {
        let store = store_seeded_with_ks(&control_root)?;
        let report = ks_pass(&store, None).await?;
        let journal = journal_keys(&store, KS_PHASE)?;
        let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
        let coaches = store.scan::<CanonicalCoach>(Table::Coaches)?;
        let counts = table_counts(&store)?;
        (journal, schools, coaches, report, counts)
    };
    let total_units = control_journal.len();
    note(
        SCENARIO,
        format!(
            "control: journal={} schools={} coaches={} requests={} from_cache={} tables={:?}",
            total_units,
            control_schools.len(),
            control_coaches.len(),
            control_report.requests,
            control_report.from_cache,
            control_counts
        ),
    );
    check!(eq; control_report.requests, 0,
    "the control run must answer the directory request from the seeded cache");
    check!(eq; control_schools.len(),
    total_units,
    "one school per journaled directory record");
    check!(total_units >= 2,
    "the fixture must carry at least two units");

    let restart_root = dir.path().join("restart");
    let first_journal = {
        let store = store_seeded_with_ks(&restart_root)?;
        let report = ks_pass(&store, Some(2)).await?;
        let journal = journal_keys(&store, KS_PHASE)?;
        note(
            SCENARIO,
            format!(
                "first pass (limit 2): journal={journal:?} rows={} requests={} from_cache={}",
                report.rows, report.requests, report.from_cache
            ),
        );
        check!(eq; journal.len(), 2);
        check!(eq; report.requests, 0);
        journal
    };

    let store = open_store(&restart_root)?;
    let resumed = ks_pass(&store, None).await?;
    let resumed_journal = journal_keys(&store, KS_PHASE)?;
    note(
        SCENARIO,
        format!(
            "restart: journal={} rows_this_pass={} notes={:?}",
            resumed_journal.len(),
            resumed.rows,
            resumed.notes
        ),
    );
    check!(eq; resumed_journal, control_journal,
    "the restart completes exactly the units the control pass covered");
    check!(eq; resumed.rows as usize,
    total_units - first_journal.len(),
    "the restart processed only the units the first pass did not journal");
    check!(resumed
        .notes
        .iter()
        .any(|entry| entry.contains("already done")),
    "the report states the resumed units were skipped: {:?}",
    resumed.notes);

    let schools = store.scan::<CanonicalSchool>(Table::Schools)?;
    let coaches = store.scan::<CanonicalCoach>(Table::Coaches)?;
    let counts = table_counts(&store)?;
    let observations = store.stats()?.observations;
    note(
        SCENARIO,
        format!(
            "merged: schools_digest={} coaches_digest={} schools={} coaches={} observations={} \
             tables={counts:?}",
            digest_of(&schools)?,
            digest_of(&coaches)?,
            schools.len(),
            coaches.len(),
            observations
        ),
    );
    check!(eq; schools, control_schools,
    "the restarted store merges to exactly the control's schools");
    check!(eq; coaches, control_coaches,
    "the restarted store merges to exactly the control's coaches");
    check!(eq; counts, control_counts,
    "observation counters match the control: no unit was written twice");
    check!(eq; count_of(&counts, Table::Schools) as usize,
    total_units,
    "one observation per journaled unit");
    Ok(())
        })
}
