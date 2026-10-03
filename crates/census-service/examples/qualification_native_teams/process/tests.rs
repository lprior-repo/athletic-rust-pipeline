use super::*;

#[test]
fn unsuccessful_reaped_child_remains_unsuccessful_at_cleanup() -> Result<()> {
    let root = tempfile::tempdir()?;
    let mut process = OwnedProcess::spawn(
        root.path(),
        "failure",
        Path::new("/usr/bin/sh"),
        &["-c".into(), "exit 7".into()],
        &[],
        Arc::new(AtomicBool::new(false)),
    )?;
    assert_eq!(process.wait(100)?.code(), Some(7));
    let cleanup = process.stop()?;
    assert_eq!(cleanup.get("success"), Some(&json!(false)));
    assert_eq!(cleanup.get("reaped"), Some(&json!(true)));
    assert_eq!(cleanup.get("already_reaped"), Some(&json!(true)));
    Ok(())
}

#[test]
fn actual_child_output_exhaustion_fails_and_reaps_without_unreadable_logs() -> Result<()> {
    let root = tempfile::tempdir()?;
    let count = super::super::artifacts::MAX_ARTIFACT
        .checked_add(1)
        .context("limit overflow")?;
    let mut process = OwnedProcess::spawn(
        root.path(),
        "overflow",
        Path::new("/usr/bin/head"),
        &["-c".into(), count.to_string(), "/dev/zero".into()],
        &[],
        Arc::new(AtomicBool::new(false)),
    )?;
    let error = process
        .wait(100)
        .err()
        .context("oversize child output accepted")?;
    assert!(format!("{error:#}").contains("resource-limit"));
    assert!(process.reaped.is_some());
    assert!(std::fs::metadata(&process.log)?.len() <= super::super::artifacts::MAX_ARTIFACT);
    let error = process
        .stop()
        .err()
        .context("exhausted child cleanup falsely certified")?;
    assert!(format!("{error:#}").contains("resource-limit"));
    Ok(())
}

#[test]
fn stdout_and_stderr_cannot_each_consume_the_whole_artifact_budget() -> Result<()> {
    let root = tempfile::tempdir()?;
    let half = super::super::artifacts::MAX_ARTIFACT / 2;
    let extra = half.checked_add(1).context("output length overflow")?;
    let script =
        format!("/usr/bin/head -c {half} /dev/zero; /usr/bin/head -c {extra} /dev/zero >&2");
    let mut process = OwnedProcess::spawn(
        root.path(),
        "both",
        Path::new("/usr/bin/sh"),
        &["-c".into(), script],
        &[],
        Arc::new(AtomicBool::new(false)),
    )?;
    let error = process
        .wait(100)
        .err()
        .context("combined output overflow accepted")?;
    assert!(format!("{error:#}").contains("resource-limit"));
    assert!(process.reaped.is_some());
    assert!(std::fs::metadata(&process.log)?.len() <= super::super::artifacts::MAX_ARTIFACT);
    assert!(read_bounded(&process.log)?.bytes().all(|byte| byte == 0));
    Ok(())
}

#[test]
fn first_stop_retains_overcap_exit_seven_before_output_finalization_fails() -> Result<()> {
    let root = tempfile::tempdir()?;
    let count = super::super::artifacts::MAX_ARTIFACT
        .checked_add(1)
        .context("limit overflow")?;
    let mut process = OwnedProcess::spawn(
        root.path(),
        "stop-overflow",
        Path::new("/usr/bin/sh"),
        &[
            "-c".into(),
            format!("/usr/bin/head -c {count} /dev/zero; exit 7"),
        ],
        &[],
        Arc::new(AtomicBool::new(false)),
    )?;
    let pid = process.pid();
    wait_until_zombie(pid)?;
    assert!(process.reaped.is_none());
    let error = process.stop().err().context("overflow cleanup accepted")?;
    assert!(format!("{error:#}").contains("resource-limit"));
    assert_eq!(
        process.reaped_status().and_then(|status| status.code()),
        Some(7)
    );
    let evidence: Value = serde_json::from_str(&read_bounded(
        &root.path().join("stop-overflow.process.json"),
    )?)?;
    assert_eq!(evidence["identity"]["pid"], json!(pid));
    assert_eq!(evidence["reaped"], json!(true));
    assert_eq!(evidence["actual_exit_status"]["code"], json!(7));
    assert_eq!(evidence["actual_exit_status"]["signal"], Value::Null);
    assert_eq!(evidence["success"], json!(false));
    assert_eq!(evidence["requested_signal"], Value::Null);
    assert!(evidence["output_error"]
        .as_str()
        .is_some_and(|error| error.contains("resource-limit")));
    assert!(std::fs::metadata(&process.log)?.len() <= super::super::artifacts::MAX_ARTIFACT);
    let ledger = read_bounded(&root.path().join("processes.jsonl"))?
        .lines()
        .map(serde_json::from_str::<Value>)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let reaped = ledger
        .iter()
        .find(|row| row["event"] == "reap")
        .context("reap evidence absent after output failure")?;
    assert_eq!(reaped["pid"], json!(pid));
    assert_eq!(reaped["evidence"]["actual_exit_status"]["code"], json!(7));
    assert_eq!(reaped["success"], json!(false));
    Ok(())
}

pub(crate) fn wait_until_zombie(pid: u32) -> Result<()> {
    let found = (0..1000).find_map(|_| {
        let state = std::fs::read_to_string(format!("/proc/{pid}/stat"));
        match state {
            Ok(stat) => {
                let zombie = stat
                    .rsplit_once(')')
                    .and_then(|(_, tail)| tail.split_whitespace().next())
                    == Some("Z");
                if zombie {
                    return Some(Ok(()));
                }
                std::thread::sleep(Duration::from_millis(10));
                None
            }
            Err(error) => Some(Err(error.into())),
        }
    });
    found.context("owned child did not exit before first stop observation")?
}
