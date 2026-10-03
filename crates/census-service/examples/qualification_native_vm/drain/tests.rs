use super::super::process::{self, Process};
use super::*;
use serde_json::json;
use std::process::Command;

const VALID: &str =
    "drained: accepted=3 completed=2 cancelled=1 timed_out=0 aborted=0 panicked=0\n";

#[test]
fn offline_read_rejects_old_certificate_after_same_boot_endpoint_restart() -> Result<()> {
    let root = tempfile::tempdir()?;
    let mut first = Process::spawn(
        root.path(),
        "serve",
        Command::new("/usr/bin/printf").arg(VALID),
    )?;
    assert_eq!(first.wait(100)?.code(), Some(0));
    let first_owner = json!({"boot_id":"isolated-boot","supervisor_pid":101,"generation":"first-supervisor","endpoint":first.identity()});
    let bytes = artifacts::read(&first.log)?;
    let certificate = json!({"identity":first_owner,"orderly":true,"endpoint_exit":first.stop()?,"counts":parse(VALID)?,"log_sha256":artifacts::sha(&bytes)});
    artifacts::publish(&root.path().join("current-owner.json"), &first_owner)?;
    artifacts::publish(&root.path().join("current-drain.json"), &certificate)?;
    assert_eq!(verify(root.path(), "isolated-boot")?, certificate);
    let mut restarted = Process::spawn(
        root.path(),
        "serve",
        Command::new("/usr/bin/printf").arg("new incarnation without drain\n"),
    )?;
    assert_eq!(restarted.wait(100)?.code(), Some(0));
    let current_owner = json!({"boot_id":"isolated-boot","supervisor_pid":102,"generation":"second-supervisor","endpoint":restarted.identity()});
    artifacts::publish(&root.path().join("current-owner.json"), &current_owner)?;
    assert!(verify(root.path(), "isolated-boot").is_err());
    assert_eq!(
        artifacts::json(&root.path().join("current-drain.json"))?,
        certificate
    );
    assert_eq!(artifacts::read(&first.log)?, VALID.as_bytes());
    assert!(parse(std::str::from_utf8(&artifacts::read(&restarted.log)?)?).is_err());
    Ok(())
}

#[test]
fn prior_certificate_is_invalidated_before_new_supervisor_spawns_endpoint() -> Result<()> {
    let root = tempfile::tempdir()?;
    let owner = json!({"boot_id":"boot","generation":"previous","endpoint":{"pid":42}});
    let certificate = json!({"identity":owner,"orderly":true});
    let starting = json!({"boot_id":"boot","generation":"new","phase":"starting"});
    artifacts::publish(&root.path().join("current-owner.json"), &starting)?;
    artifacts::publish(&root.path().join("current-drain.json"), &certificate)?;
    assert!(verify(root.path(), "boot").is_err());
    Ok(())
}

#[test]
fn balanced_terminal_counts_are_accepted_and_unfinished_or_bad_outcomes_are_refused() -> Result<()>
{
    let counts = parse(VALID)?;
    assert_eq!(
        serde_json::to_value(counts)?,
        json!({"accepted":3,"completed":2,"cancelled":1,"timed_out":0,"aborted":0,"panicked":0})
    );
    [
        "prefix drained: accepted=3 completed=2 cancelled=1 timed_out=0 aborted=0 panicked=0",
        "drained: accepted=3 completed=1 cancelled=1 timed_out=0 aborted=0 panicked=0",
        "drained: accepted=3 completed=2 cancelled=1 timed_out=1 aborted=0 panicked=0",
        "drained: accepted=3 completed=2 cancelled=1 timed_out=0 aborted=1 panicked=0",
        "drained: accepted=3 completed=2 cancelled=1 timed_out=0 aborted=0 panicked=1",
        "drained: accepted=0 completed=18446744073709551615 cancelled=1 timed_out=0 aborted=0 panicked=0",
        "drained: accepted=3 completed=2 cancelled=1 timed_out=0 aborted=0 panicked=0 extra=1",
        "drained: accepted=3 completed=2 cancelled=1 timed_out=0 aborted=0 panicked=0\ndrained: accepted=0 completed=0 cancelled=0 timed_out=0 aborted=0 panicked=0",
    ].into_iter().for_each(|input| assert!(parse(input).is_err(), "{input}"));
    Ok(())
}

#[test]
fn successful_old_log_cannot_certify_current_endpoint_signal_death() -> Result<()> {
    let root = tempfile::tempdir()?;
    let log = root.path().join("current.log");
    artifacts::write(&log, VALID.as_bytes())?;
    let identity = json!({"pid":42,"generation":"current","log":log});
    let owner = json!({"boot_id":"boot","generation":"supervisor","endpoint":identity});
    let certificate = json!({"identity":owner,"orderly":true,"endpoint_exit":{"identity":identity,"reaped":true,"exit":{"code":null,"signal":10}},"counts":parse(VALID)?,"log_sha256":artifacts::sha(VALID.as_bytes())});
    artifacts::publish(&root.path().join("current-owner.json"), &owner)?;
    artifacts::publish(&root.path().join("current-drain.json"), &certificate)?;
    assert!(verify(root.path(), "boot").is_err());
    Ok(())
}

#[test]
fn same_label_command_outputs_are_isolated_between_incarnations() -> Result<()> {
    let root = tempfile::tempdir()?;
    let first = process::command(
        root.path(),
        "same-label",
        Command::new("/usr/bin/printf").arg("first"),
        100,
    )?;
    let second = process::command(
        root.path(),
        "same-label",
        Command::new("/usr/bin/printf").arg("second"),
        100,
    )?;
    assert_eq!(first, "first");
    assert_eq!(second, "second");
    Ok(())
}
