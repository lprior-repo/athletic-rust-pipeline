use super::discovery::{
    discovery_client, drain_line, endpoint_answers, free_loopback_port, parse_drain_line,
    record_discovery_services, send_sigterm, spawn_serve, stop_reason_line, wait_for_exit,
};
use super::material::process_units;
use super::{
    digest_of, journal_keys, note, open_store, table_counts, CanonicalSchool, Duration,
    ExitStatusExt, Table, OBSERVED_ON, UNIT_PHASE,
};

#[test]
fn sigkill_of_the_service_keeps_durable_work_and_the_restart_drains_cleanly() -> super::TestResult {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?
        .block_on(async {
            const SCENARIO: &str = "sigkill-service";
            let dir = tempfile::tempdir()?;
            let root = dir.path().join("store");
            let units = ["unit-1", "unit-2", "unit-3"];

            let (journal_before, counts_before, merged_digest_before) = {
                let store = open_store(&root)?;
                process_units(&store, &units, OBSERVED_ON)?;
                store.flush()?;
                let journal = journal_keys(&store, UNIT_PHASE)?;
                let counts = table_counts(&store)?;
                let merged = store.scan::<CanonicalSchool>(Table::Schools)?;
                (journal, counts, digest_of(&merged)?)
            };
            note(
                SCENARIO,
                format!("before the kill: journal={journal_before:?} tables={counts_before:?}"),
            );

            let port = free_loopback_port()?;
            let (mut child, manifest) = spawn_serve(&root, port, 30).await?;
            record_discovery_services(SCENARIO, &manifest)?;
            check!(
                child.try_wait()?.is_none(),
                "the service must still be running when it is killed"
            );

            child.kill()?;
            let status = child.wait()?;
            note(
                SCENARIO,
                format!(
                    "service killed: status={status:?} signal={:?}",
                    status.signal()
                ),
            );
            check!(eq; status.signal(),
    Some(9),
    "the service was killed hard, with no drain");

            let store = open_store(&root)?;
            let journal_after_kill = journal_keys(&store, UNIT_PHASE)?;
            let counts_after_kill = table_counts(&store)?;
            let merged_after_kill = digest_of(&store.scan::<CanonicalSchool>(Table::Schools)?)?;
            note(
                SCENARIO,
                format!(
                    "after the kill: journal={} tables={counts_after_kill:?}",
                    journal_after_kill.len()
                ),
            );
            check!(eq; journal_after_kill, journal_before,
    "a SIGKILL cannot lose journaled work or invent it");
            check!(eq; counts_after_kill, counts_before,
    "the reopened store holds exactly the counters it had before the kill");
            check!(eq; merged_after_kill, merged_digest_before,
    "the merged rows are unchanged by the kill");
            drop(store);

            for restart in 1..=2 {
                let (child, manifest) = spawn_serve(&root, port, 30).await?;
                record_discovery_services(SCENARIO, &manifest)?;
                note(SCENARIO, format!("restart {restart} is up"));
                send_sigterm(&child)?;
                let output = child.wait_with_output()?;
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let counters = parse_drain_line(drain_line(&stdout));
                note(
                    SCENARIO,
                    format!(
                        "restart {restart} drain report: {} ({})",
                        drain_line(&stdout),
                        stop_reason_line(&stdout)
                    ),
                );
                check!(
                    output.status.success(),
                    "a drained service exits cleanly: {:?}\nstdout:\n{stdout}",
                    output.status
                );
                check!(
                    stdout.contains("stop_reason: Signal"),
                    "the drain was the operator's SIGTERM, not a server exit:\n{stdout}"
                );
                check!(eq; counters.get("panicked"), Some(&0));
                check!(eq; counters.get("accepted"),
        Some(
            &(counters.get("completed").copied().map_or(0, |value| value)
                + counters.get("cancelled").copied().map_or(0, |value| value)
                + counters.get("aborted").copied().map_or(0, |value| value))
        ),
        "the terminal counters account for every accepted task: {}",
        drain_line(&stdout));

                let store = open_store(&root)?;
                let counts_after_drain = table_counts(&store)?;
                let merged_after_drain =
                    digest_of(&store.scan::<CanonicalSchool>(Table::Schools)?)?;
                drop(store);
                note(
                    SCENARIO,
                    format!("restart {restart} tables after the drain: {counts_after_drain:?}"),
                );
                check!(eq; counts_after_drain, counts_before,
        "the drain and finalize left the counters where the kill did");
                check!(eq; merged_after_drain, merged_digest_before,
        "a restart and a drain do not rewrite merged rows");
            }
            Ok(())
        })
}

#[test]
fn service_with_no_stop_request_survives_its_drain_deadline() -> super::TestResult {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?
        .block_on(async {
    const SCENARIO: &str = "drain-deadline";
    const DRAIN_SECS: u64 = 2;
    let dir = tempfile::tempdir()?;
    let root = dir.path().join("store");
    let units = ["unit-1", "unit-2"];

    let (journal_before, counts_before) = {
        let store = open_store(&root)?;
        process_units(&store, &units, OBSERVED_ON)?;
        store.flush()?;
        (journal_keys(&store, UNIT_PHASE)?, table_counts(&store)?)
    };
    note(
        SCENARIO,
        format!("before the service: journal={journal_before:?} tables={counts_before:?}"),
    );

    let port = free_loopback_port()?;
    let (mut child, manifest) = spawn_serve(&root, port, DRAIN_SECS).await?;
    record_discovery_services(SCENARIO, &manifest)?;

    let exited = wait_for_exit(&mut child, Duration::from_secs(DRAIN_SECS + 8))?;
    let still_answering = endpoint_answers(&discovery_client()?, port).await;
    match exited {
        Some(elapsed) => {
            let output = child.wait_with_output()?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            note(
                SCENARIO,
                format!(
                    "DEFECT: no stop request was sent, yet the process exited after {elapsed:?} \
                     (drain-timeout {DRAIN_SECS}s) with status {:?} and still_answering={still_answering}: \
                     an unrequested stop ends the service. {} => {}",
                    output.status,
                    stop_reason_line(&stdout),
                    drain_line(&stdout)
                ),
            );
        }
        None if !still_answering => {
            send_sigterm(&child)?;
            let output = child.wait_with_output()?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            note(
                SCENARIO,
                format!(
                    "DEFECT: the process outlived its drain deadline, but the endpoint stopped \
                     answering `/discover` with no stop request: the drain deadline reaped a healthy \
                     endpoint instead of waiting for the stop that is supposed to trigger it. \
                     status={:?} {} => {}",
                    output.status,
                    stop_reason_line(&stdout),
                    drain_line(&stdout)
                ),
            );
        }
        None => {
            note(
                SCENARIO,
                format!(
                    "still serving after the drain deadline with no stop request, \
                     still_answering={still_answering}"
                ),
            );
            send_sigterm(&child)?;
            let output = child.wait_with_output()?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            note(
                SCENARIO,
                format!(
                    "stopped by the operator: status={:?} {}",
                    output.status,
                    stop_reason_line(&stdout)
                ),
            );
            check!(output.status.success(),
            "a drained service exits cleanly: {:?}\nstdout:\n{stdout}",
            output.status);
            check!(stdout.contains("stop_reason: Signal"),
            "the stop was the operator's SIGTERM, not a server exit:\n{stdout}");
        }
    }

    let store = open_store(&root)?;
    let journal_after = journal_keys(&store, UNIT_PHASE)?;
    let counts_after = table_counts(&store)?;
    note(
        SCENARIO,
        format!(
            "after the service: journal={} tables={counts_after:?}",
            journal_after.len()
        ),
    );
    check!(eq; journal_after, journal_before,
    "an unrequested stop cannot change the journal");
    check!(eq; counts_after, counts_before,
    "an unrequested stop cannot change the counters");
    Ok(())
        })
}
