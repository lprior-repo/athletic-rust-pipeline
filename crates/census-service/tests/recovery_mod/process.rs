use super::{
    count_of, note, open_store, table_counts, BTreeMap, BTreeSet, Command, Duration, ExitStatusExt,
    Instant, Output, Path, Stdio, Store, Table, CENSUS_BIN, DEAD_PROXY,
};

fn census_command(root: &Path) -> Command {
    let mut command = Command::new(CENSUS_BIN);
    command
        .arg("--store")
        .arg(root)
        .arg("--delay-ms")
        .arg("0")
        .env("RUST_LOG", "off")
        .env("HTTPS_PROXY", DEAD_PROXY)
        .env("HTTP_PROXY", DEAD_PROXY);
    command
}

pub(super) fn run_census(root: &Path, args: &[&str]) -> super::TestResult<Output> {
    let output = census_command(root).args(args).output()?;
    check!(
        output.status.success(),
        "census-service {args:?} failed with {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    Ok(output)
}

pub(super) fn report_of(output: &Output) -> super::TestResult<serde_json::Value> {
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(serde_json::from_str(&stdout)?)
}

pub(super) fn counters_of(output: &Output) -> BTreeMap<String, u64> {
    let mut allowed: BTreeSet<&str> = Table::ALL.iter().map(|table| table.file()).collect();
    allowed.insert("observations");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .filter(|(name, _)| allowed.contains(*name))
        .filter_map(|(name, count)| {
            count
                .trim()
                .parse::<u64>()
                .ok()
                .map(|count| (name.to_string(), count))
        })
        .collect()
}

pub(super) struct KillAttempt {
    delay: Duration,
    exited_before_kill: bool,
    signal: Option<i32>,
    pub(super) receipts: u64,
    pub(super) entity_ids: BTreeSet<String>,
    observations: u64,
}

fn attempt_kill(
    root: &Path,
    args: &[&str],
    delay: Duration,
    table: Table,
    ids_of: fn(&Store) -> super::TestResult<BTreeSet<String>>,
) -> super::TestResult<KillAttempt> {
    let mut child = census_command(root)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    std::thread::sleep(delay);
    let exited_before_kill = child.try_wait()?.is_some();
    let _ = child.kill();
    let status = child.wait()?;
    let store = open_store(root)?;
    let receipts = store.receipt_count()?;
    let entity_ids = ids_of(&store)?;
    let observations = count_of(&table_counts(&store)?, table);
    Ok(KillAttempt {
        delay,
        exited_before_kill,
        signal: status.signal(),
        receipts,
        entity_ids,
        observations,
    })
}

pub(super) struct Subject {
    pub(super) table: Table,
    pub(super) ids_of: fn(&Store) -> super::TestResult<BTreeSet<String>>,
}

const LADDER_ROUNDS: u64 = 8;

const LADDER_OFFSETS_US: [u64; 14] = [
    50, 100, 200, 300, 500, 750, 1_000, 1_500, 2_000, 4_000, 8_000, 16_000, 24_000, 32_000,
];

fn reset_attempt_database(root: &Path) -> super::TestResult {
    let database = root.join("fjall");
    if database.exists() {
        std::fs::remove_dir_all(database)?;
    }
    Ok(())
}

fn measure_clean_runtime(root: &Path, args: &[&str]) -> super::TestResult<u64> {
    reset_attempt_database(root)?;
    let started = Instant::now();
    let status = census_command(root)
        .args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    check!(status.success(), "a clean pass must exit zero: {status:?}");
    Ok(u64::try_from(started.elapsed().as_micros()).map_or(5_000_000, |value| value))
}

pub(super) fn kill_ladder(
    scenario: &str,
    root: &Path,
    args: &[&str],
    subject: Subject,
    total_units: usize,
    clean_runtime: Duration,
) -> super::TestResult<Vec<KillAttempt>> {
    note(
        scenario,
        format!(
            "clean runtime {clean_runtime:?} before the ladder; every round re-measures it and \
             never anchors above a delay already known to let the pass finish, so a machine whose \
             pass time drifts under load still probes the batch boundary"
        ),
    );
    let mut anchor_us = u64::try_from(clean_runtime.as_micros()).map_or(u64::MAX, |value| value);
    let mut attempts = Vec::new();
    for round in 0..LADDER_ROUNDS {
        let measured_us = measure_clean_runtime(root, args)?;
        let runtime_us = measured_us.min(anchor_us).max(1);
        note(
            scenario,
            format!("round {round}: clean runtime {measured_us} us, anchor {runtime_us} us"),
        );
        for offset_us in LADDER_OFFSETS_US {
            reset_attempt_database(root)?;
            let delay_us = runtime_us.saturating_sub(offset_us).max(1);
            let attempt = attempt_kill(
                root,
                args,
                Duration::from_micros(delay_us),
                subject.table,
                subject.ids_of,
            )?;
            note(scenario, format!(
                "round {round} offset={offset_us} us kill delay={:?} exited_before_kill={} signal={:?} receipts={} entities={} observations={}",
                attempt.delay, attempt.exited_before_kill, attempt.signal,
                attempt.receipts, attempt.entity_ids.len(), attempt.observations,
            ));
            let partial = !attempt.entity_ids.is_empty() && attempt.entity_ids.len() < total_units;
            if partial {
                check!(
                    !attempt.exited_before_kill,
                    "partial work must be interrupted while running"
                );
                check!(eq; attempt.signal,
                Some(9),
                "partial work must be interrupted by SIGKILL");
            }
            let completed = attempt.entity_ids.len() >= total_units;
            if let Some(completed_us) = completed
                .then(|| u64::try_from(attempt.delay.as_micros()).map_or(u64::MAX, |value| value))
            {
                anchor_us = anchor_us.min(completed_us);
            }
            attempts.push(attempt);
            if partial {
                note(
                    scenario,
                    "partial SIGKILL landed at a durable commit boundary",
                );
                return Ok(attempts);
            }
        }
    }
    Ok(attempts)
}
