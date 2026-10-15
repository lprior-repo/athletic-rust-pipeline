use super::*;
use census_store::publish::{Root, State};
use std::io::Write;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};
mod harness;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn native_publication_worker() -> TestResult {
    let Some(store) = std::env::var_os("CENSUS_PUBLICATION_CHILD_STORE") else {
        return Ok(());
    };
    let store = Store::open(PathBuf::from(store))?;
    let mut options = super::fence_tests::options(None)?;
    options.limit = Some(1);
    let dataset = ExportDataset::for_job(&store, "crash-window")?;
    let censuses = crate::workbook::Censuses::of(&dataset, &store.out_dir(), &[]);
    crate::workbook::build_from(&dataset, &store, &options, &censuses)?;
    Err("crash worker returned without terminating at its checkpoint".into())
}

#[test]
fn publication_fence_kill_after_intent_preserves_old_output_and_retries() -> TestResult {
    crash("intent", RootSelection::Expected, Termination::Killed)
}
#[test]
fn publication_fence_kill_after_rename_before_fsync_retains_visible_bundle() -> TestResult {
    crash("renamed", RootSelection::Desired, Termination::Killed)
}

#[test]
fn publication_fence_abort_after_intent_recovers_byte_identical_bundle() -> TestResult {
    crash("intent", RootSelection::Expected, Termination::Aborted)
}

#[test]
fn publication_fence_abort_after_rename_recovers_byte_identical_bundle() -> TestResult {
    crash("renamed", RootSelection::Desired, Termination::Aborted)
}

enum Termination {
    Killed,
    Aborted,
}

enum RootSelection {
    Expected,
    Desired,
}
fn crash(window: &str, selection: RootSelection, termination: Termination) -> TestResult {
    let temp = tempfile::tempdir()?;
    let store_path = temp.path().join("store");
    let store = Store::open(&store_path)?;
    let options = super::fence_tests::options(None)?;
    let old = crate::workbook::build(&store, &options)?;
    let bytes = std::fs::read(&old)?;
    let root = store.out_dir().join("publication");
    let expected = pointer::read(&root)?;
    drop(store);
    let socket_directory = tempfile::tempdir_in("/tmp")?;
    let socket_path = socket_directory.path().join("crash.sock");
    let listener = std::os::unix::net::UnixListener::bind(&socket_path)?;
    let mut command = Command::new(std::env::current_exe()?);
    command
        .args([
            "--exact",
            "workbook::publication::crash_tests::native_publication_worker",
            "--nocapture",
        ])
        .env("CENSUS_PUBLICATION_CHILD_STORE", &store_path)
        .env("CENSUS_PUBLICATION_CRASH_WINDOW", window)
        .stdin(Stdio::null())
        .env("CENSUS_PUBLICATION_CRASH_SOCKET", &socket_path)
        .env(
            "CENSUS_PUBLICATION_CRASH_TERMINATION",
            match termination {
                Termination::Killed => "kill",
                Termination::Aborted => "abort",
            },
        )
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    let status = harness::with_child(command, |child| {
        let mut checkpoint =
            harness::wait_checkpoint(listener, child, window, harness::WaitBudget::REAL)?;
        match termination {
            Termination::Killed => child.kill()?,
            Termination::Aborted => checkpoint.write_all(&[1])?,
        }
        Ok(child.wait()?)
    })?;
    let signal = match termination {
        Termination::Killed => 9,
        Termination::Aborted => 6,
    };
    check!(eq; status.signal(), Some(signal));
    std::fs::remove_file(&socket_path)?;
    let store = Store::open(&store_path)?;
    let namespace = binding::namespace(&root)?;
    let intent = store
        .publication_active(&namespace)?
        .ok_or("durable intent missing")?;
    let desired = intent.desired().ok_or("desired digest missing")?.clone();
    let desired_directory = root.join("generations").join(desired.as_str());
    let desired_bytes = bundle_bytes(&desired_directory)?;
    check!(eq; intent.state, State::Intent { desired: desired.clone() });
    match selection {
        RootSelection::Expected => check!(eq; pointer::read(&root)?, expected),
        RootSelection::Desired => check!(eq; pointer::read(&root)?, Root::Bundle(desired.clone())),
    }
    check!(eq; std::fs::read(&old)?, bytes);
    verify_published(&old)?;
    let outcome = recover_publication(&store, &root)?;
    let expected_outcome = match selection {
        RootSelection::Expected => RecoveryOutcome::ExpectedRetried {
            bundle: desired.clone(),
        },
        RootSelection::Desired => RecoveryOutcome::DesiredPublished {
            bundle: desired.clone(),
        },
    };
    check!(eq; outcome, expected_outcome);
    check!(eq; pointer::read(&root)?, Root::Bundle(desired.clone()));
    check!(eq; store.publication_read(&intent.request.identity)?.ok_or("receipt missing")?.state, State::Published { receipt: desired.clone() });
    let verified = verify_published(&desired_directory.join("workbook.xlsx"))?;
    check!(eq; verified.generation_digest, desired.as_str());
    check!(eq; bundle_bytes(&desired_directory)?, desired_bytes);
    check!(eq; recover_publication(&store, &root)?, RecoveryOutcome::HistoricalSuccess { bundle: desired.clone() });
    check!(eq; pointer::read(&root)?, Root::Bundle(desired));
    check!(eq; bundle_bytes(&desired_directory)?, desired_bytes);
    eprintln!(
        "publication smoke checkpoint={window} signal={signal} bundle={} artifacts={} byte_identical=true recovery={expected_outcome:?} repeated=HistoricalSuccess",
        verified.generation_digest,
        desired_bytes.len()
    );
    check!(eq; std::fs::read(&old)?, bytes);
    verify_published(&old)?;
    Ok(())
}

fn bundle_bytes(directory: &Path) -> TestResult<std::collections::BTreeMap<PathBuf, Vec<u8>>> {
    std::fs::read_dir(directory)?
        .map(|entry| {
            let entry = entry?;
            Ok((entry.file_name().into(), std::fs::read(entry.path())?))
        })
        .collect()
}
