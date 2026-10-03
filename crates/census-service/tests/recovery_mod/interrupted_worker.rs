use super::process::{counters_of, kill_ladder, report_of, run_census, Subject};
use super::{
    count_of, journal_keys, meet_ids, note, open_store, table_counts, Instant, Table,
    ATHLETICLIVE_FIXTURE, ATHLETICLIVE_PHASE,
};

fn generate_athleticlive_csv(count: usize) -> super::TestResult<String> {
    let mut csv = format!(
        "{}\n",
        ATHLETICLIVE_FIXTURE
            .lines()
            .next()
            .ok_or("missing CSV header")?
    );
    for i in 0..count {
        csv.push_str(&format!(
            "athleticlive,{},,State Meet {i},,Illinois,2025-09-15T04:00:00Z,,True\n",
            50000 + i + 1
        ));
    }
    Ok(csv)
}

#[test]
fn sigkill_mid_batch_worker_restart_completes_the_remaining_units() -> super::TestResult {
    const SCENARIO: &str = "sigkill-worker";
    let dir = tempfile::tempdir()?;
    let input = dir.path().join("athleticlive-meets.csv");
    std::fs::write(&input, generate_athleticlive_csv(4096)?)?;
    let args = [
        "provider",
        "athleticlive",
        "--input",
        input.to_str().ok_or("CSV path is not UTF-8")?,
    ];

    let control_root = dir.path().join("control");
    let started = Instant::now();
    run_census(&control_root, &args)?;
    let clean_runtime = started.elapsed();
    let control_stats = counters_of(&run_census(&control_root, &["fjall-stats"])?);
    let (control_journal, control_ids) = {
        let store = open_store(&control_root)?;
        (journal_keys(&store, ATHLETICLIVE_PHASE)?, meet_ids(&store)?)
    };
    let total_units = control_journal.len();
    note(
        SCENARIO,
        format!("control: runtime={clean_runtime:?} units={total_units} stats={control_stats:?}"),
    );
    check!(eq; control_stats.get("meets").copied(),
    Some(total_units as u64),
    "one meet row per journaled meet");

    let root = dir.path().join("killed");
    let attempts = kill_ladder(
        SCENARIO,
        &root,
        &args,
        Subject {
            phase: ATHLETICLIVE_PHASE,
            table: Table::Meets,
            ids_of: meet_ids,
        },
        total_units,
        clean_runtime,
    )?;
    let landed_partial = attempts
        .iter()
        .any(|attempt| !attempt.journal.is_empty() && attempt.journal.len() < total_units);
    check!(
        landed_partial,
        "a real mid-batch kill must have happened (0 < journal < total); workload completed \
     before the first kill delay"
    );
    for attempt in &attempts {
        check!(eq; attempt.journal, attempt.entity_ids,
        "each meet and its journal marker must commit atomically");
    }

    let resumed = report_of(&run_census(&root, &args)?)?;
    let (journal, ids) = {
        let store = open_store(&root)?;
        (journal_keys(&store, ATHLETICLIVE_PHASE)?, meet_ids(&store)?)
    };
    let observations = {
        let store = open_store(&root)?;
        count_of(&table_counts(&store)?, Table::Meets)
    };
    let resumed_stats = counters_of(&run_census(&root, &["fjall-stats"])?);
    let consolidated = counters_of(&run_census(&root, &["consolidate"])?);
    note(
        SCENARIO,
        format!(
            "restart: rows={} journal={} meets={} meet_observations={observations} \
             stats={resumed_stats:?} consolidate={consolidated:?}",
            resumed["rows"],
            journal.len(),
            ids.len()
        ),
    );
    note(
        SCENARIO,
        format!(
            "partial_kill_landed={landed_partial} attempts={} observations_delta={}",
            attempts.len(),
            observations
                .saturating_sub(control_stats.get("meets").copied().map_or(0, |value| value))
        ),
    );

    check!(eq; journal, control_journal,
    "the restart completes exactly the control's unit set");
    check!(eq; ids, control_ids,
    "the killed-then-restarted store merges to the control's meets: no lost unit, no double count");
    check!(eq; consolidated.get("meets"),
    control_stats.get("meets"),
    "the snapshot holds one row per meet, not one per observation");
    check!(eq; observations, total_units as u64,
    "an atomic append-and-journal batch cannot replay an acknowledged observation");
    check!(eq; resumed_stats, control_stats,
    "all durable counters must match the clean run");
    Ok(())
}
