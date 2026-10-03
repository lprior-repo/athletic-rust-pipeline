use super::*;

mod resources;
mod signals;

#[test]
fn prior_failed_exit_remains_failed_after_reap_and_repeated_stop() -> Result<()> {
    let root = tempfile::tempdir()?;
    let mut child = Process::spawn(
        root.path(),
        "failed",
        Command::new("/bin/sh").args(["-c", "exit 7"]),
    )?;
    assert_eq!(child.wait(100)?.code(), Some(7));
    assert!(child.stop().is_err());
    assert!(child.stop().is_err());
    let evidence = child.evidence();
    assert_eq!(evidence.get("reaped"), Some(&json!(true)));
    assert_eq!(evidence.pointer("/exit/code"), Some(&json!(7)));
    assert_eq!(evidence.get("requested_signal"), Some(&Value::Null));
    Ok(())
}

#[test]
fn actual_unexpected_signal_is_not_orderly_even_after_reap() -> Result<()> {
    let root = tempfile::tempdir()?;
    let mut child = Process::spawn(
        root.path(),
        "signal",
        Command::new("/bin/sh").args(["-c", "kill -USR1 $$"]),
    )?;
    assert_eq!(child.wait(100)?.signal(), Some(10));
    assert!(child.stop().is_err());
    let evidence = child.evidence();
    assert_eq!(evidence.pointer("/exit/signal"), Some(&json!(10)));
    assert_eq!(evidence.get("reaped"), Some(&json!(true)));
    assert_eq!(evidence.get("requested_signal"), Some(&Value::Null));
    Ok(())
}

#[test]
fn different_signal_during_requested_term_is_not_orderly() -> Result<()> {
    let root = tempfile::tempdir()?;
    let script = "trap 'kill -USR1 $$' TERM; printf ready; i=0; while [ \"$i\" -lt 60 ]; do /usr/bin/sleep 1; i=$((i+1)); done";
    let mut child = Process::spawn(
        root.path(),
        "term-crash",
        Command::new("/bin/sh").args(["-c", script]),
    )?;
    (0..1000)
        .find_map(|_| match artifacts::read(&child.log) {
            Ok(bytes) if bytes == b"ready" => Some(Ok::<_, anyhow::Error>(())),
            Err(error) => Some(Err(error)),
            Ok(_) => {
                std::thread::sleep(Duration::from_millis(10));
                None
            }
        })
        .context("signal trap fixture did not become ready")??;
    assert!(child.stop().is_err());
    let evidence = child.evidence();
    assert_eq!(evidence.get("requested_signal"), Some(&json!(15)));
    assert_eq!(evidence.pointer("/exit/signal"), Some(&json!(10)));
    assert_eq!(evidence.get("reaped"), Some(&json!(true)));
    Ok(())
}
#[test]
fn requested_term_retains_exact_signal_and_reap_identity() -> Result<()> {
    let root = tempfile::tempdir()?;
    let mut child = Process::spawn(
        root.path(),
        "sleep",
        Command::new("/usr/bin/sleep").arg("60"),
    )?;
    let identity = child.identity();
    let evidence = child.stop()?;
    assert_eq!(evidence.get("identity"), Some(&identity));
    assert_eq!(evidence.get("requested_signal"), Some(&json!(15)));
    assert_eq!(evidence.pointer("/exit/signal"), Some(&json!(15)));
    assert_eq!(evidence.get("reaped"), Some(&json!(true)));
    assert_eq!(child.stop()?, evidence);
    Ok(())
}
