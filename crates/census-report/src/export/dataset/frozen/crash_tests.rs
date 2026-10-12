use super::super::fixtures::{athlete, school, seeded_store, TestResult};
use super::super::{freeze_v2, to_export_dataset};
use census_store::{InputState, Store, Table};
use std::io::{BufRead, Write};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

const CHILD: &str = "export::dataset::frozen::freeze::crash_tests::freeze_crash_child";
const JOB: &str = "freeze-crash-matrix";
static STEP: AtomicUsize = AtomicUsize::new(0);

pub(super) fn checkpoint(name: &str) -> TestResult {
    if std::env::var_os("FREEZE_CRASH_STORE").is_none() {
        return Ok(());
    }
    let step = STEP.fetch_add(1, Ordering::Relaxed);
    println!("FREEZE_STEP {step} {name}");
    std::io::stdout().flush()?;
    if std::env::var("FREEZE_CRASH_STEP").ok().as_deref() == Some(step.to_string().as_str()) {
        match std::env::var("FREEZE_CRASH_MODE")?.as_str() {
            "crash" => std::process::exit(86),
            "kill" => loop {
                std::thread::park();
            },
            _ => return Err("unknown crash mode".into()),
        }
    }
    Ok(())
}

#[test]
fn freeze_crash_child() -> TestResult {
    let Some(directory) = std::env::var_os("FREEZE_CRASH_STORE") else {
        return Ok(());
    };
    let store = Store::open(directory)?;
    let root = freeze_v2(&store, JOB)?;
    check!(eq; store.get_input_record(root.attempt())?.map(|r| r.state()), Some(InputState::Frozen));
    Ok(())
}

fn child(directory: &std::path::Path) -> TestResult<Command> {
    let mut command = Command::new(std::env::current_exe()?);
    command.args(["--exact", CHILD, "--nocapture"]);
    command.env("FREEZE_CRASH_STORE", directory);
    command.stdout(Stdio::piped());
    command.stderr(Stdio::inherit());
    Ok(command)
}

#[test]
fn freeze_crash_matrix_recovers_each_durable_step() -> TestResult {
    let seeded = seeded_store()?;
    drop(seeded.store);
    let output = child(seeded.directory.path())?.output()?;
    check!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    let steps = stdout
        .lines()
        .filter(|line| line.starts_with("FREEZE_STEP "))
        .count();
    check!(
        steps >= 20,
        "the matrix must include object, root and durable record transitions"
    );
    for mode in ["crash", "kill"] {
        for step in 0..steps {
            exercise(mode, step)?;
        }
    }
    println!("freeze crash matrix: {steps} durable steps x 2 modes recovered");
    Ok(())
}

fn exercise(mode: &str, step: usize) -> TestResult {
    let seeded = seeded_store()?;
    let old = super::super::super::super::ExportDataset::load(&seeded.store)?;
    drop(seeded.store);
    let mut command = child(seeded.directory.path())?;
    command
        .env("FREEZE_CRASH_STEP", step.to_string())
        .env("FREEZE_CRASH_MODE", mode);
    let mut process = command.spawn()?;
    let stdout = process.stdout.take().ok_or("child stdout missing")?;
    let marker = format!("FREEZE_STEP {step} ");
    let mut reached = false;
    for line in std::io::BufReader::new(stdout).lines() {
        if line?.starts_with(&marker) {
            reached = true;
            break;
        }
    }
    if mode == "kill" && reached {
        process.kill()?;
    }
    let status = process.wait()?;
    check!(reached, "the child did not reach the requested step");
    check!(!status.success());
    if mode == "crash" {
        check!(eq; status.code(), Some(86));
    }
    let store = Store::open(seeded.directory.path())?;
    let added = school("After Crash High");
    store.append(Table::Schools, &added)?;
    store.append(
        Table::Athletes,
        &athlete(&added.id, "After Crash Runner", "after-crash"),
    )?;
    let new = super::super::super::super::ExportDataset::load(&store)?;
    let root = freeze_v2(&store, JOB)?;
    let recovered = to_export_dataset(&root, &store)?;
    check!(
        recovered.lineage.input_digest == old.lineage.input_digest
            || recovered.lineage.input_digest == new.lineage.input_digest,
        "recovery spliced snapshot generations"
    );
    let record = store
        .get_input_record(root.attempt())?
        .ok_or("recovered root has no record")?;
    check!(eq; record.state(), InputState::Frozen);
    let epoch = store.lease_acquire(root.attempt(), "crash-lock-probe", "2999-12-31")?;
    store.lease_release(root.attempt(), epoch)?;
    let again = freeze_v2(&store, JOB)?;
    check!(eq; again.attempt(), root.attempt());
    Ok(())
}
