use super::process::{counters_of, kill_ladder, report_of, run_census, Subject};
use super::{
    count_of, meet_ids, note, open_store, table_counts, Instant, Table, ATHLETICLIVE_FIXTURE,
};
use sha2::{Digest, Sha256};

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
            "athleticlive,{},,Synthetic Recovery Meet {i},,Illinois,2025-09-15T04:00:00Z,,True\n",
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
    let body = generate_athleticlive_csv(4096)?;
    std::fs::write(&input, &body)?;
    let metadata = dir.path().join("synthetic-capture.meta.json");
    let producer = serde_json::json!({
        "url": "https://synthetic.athleticlive.example/meets.csv",
        "method": "GET", "status": 200, "bytes": body.len(),
        "content_digest": format!("{:x}", Sha256::digest(body.as_bytes())),
        "fetched_at": "2026-09-20T00:00:00Z", "content_type": "text/csv"
    });
    std::fs::write(&metadata, serde_json::to_vec(&producer)?)?;
    let args = [
        "provider",
        "athleticlive",
        "--input",
        input.to_str().ok_or("CSV path is not UTF-8")?,
        "--input-metadata",
        metadata.to_str().ok_or("metadata path is not UTF-8")?,
    ];

    let control_root = dir.path().join("control");
    let started = Instant::now();
    run_census(&control_root, &args)?;
    let clean_runtime = started.elapsed();
    let control_stats = counters_of(&run_census(&control_root, &["fjall-stats"])?);
    let (control_receipts, control_ids) = {
        let store = open_store(&control_root)?;
        (store.receipt_count()?, meet_ids(&store)?)
    };
    let total_units = control_ids.len();
    check!(eq; total_units, 4096);
    check!(eq; control_receipts, u64::try_from(total_units)?);
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
            table: Table::Meets,
            ids_of: meet_ids,
        },
        total_units,
        clean_runtime,
    )?;
    let landed_partial = attempts
        .iter()
        .any(|attempt| !attempt.entity_ids.is_empty() && attempt.entity_ids.len() < total_units);
    check!(
        landed_partial,
        "a real mid-batch kill must have happened (0 < durable units < total); workload completed \
     before the first kill delay"
    );
    for attempt in &attempts {
        check!(eq; attempt.receipts, u64::try_from(attempt.entity_ids.len())?,
            "each physical meet effect and its receipt commit atomically");
    }

    let resumed = report_of(&run_census(&root, &args)?)?;
    let (receipts, ids) = {
        let store = open_store(&root)?;
        (store.receipt_count()?, meet_ids(&store)?)
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
            "restart: rows={} receipts={} meets={} meet_observations={observations} \
             stats={resumed_stats:?} consolidate={consolidated:?}",
            resumed["rows"],
            receipts,
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

    check!(eq; receipts, control_receipts,
    "the restart completes exactly the control's durable effect set");
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
