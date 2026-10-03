use super::super::*;

#[test]
fn stdout_and_stderr_resource_overruns_leave_bounded_failure_and_reaped_child() -> Result<()> {
    [false, true]
        .into_iter()
        .try_for_each(|stderr| -> Result<()> {
            let root = tempfile::tempdir()?;
            let bytes = artifacts::LIMIT
                .checked_add(8192)
                .context("log test budget overflow")?;
            let script = format!(
                "exec /usr/bin/head -c {bytes} /dev/zero{}",
                if stderr { " >&2" } else { "" }
            );
            let mut child = Process::spawn(
                root.path(),
                "overrun",
                Command::new("/bin/sh").args(["-c", &script]),
            )?;
            assert!(child.wait(300).is_err());
            assert!(child.stop().is_err());
            assert_eq!(child.evidence().get("reaped"), Some(&json!(true)));
            let path = if stderr {
                child
                    .log
                    .with_file_name(format!("overrun-{}.stderr.log", child.generation))
            } else {
                child.log.clone()
            };
            let retained = artifacts::read(&path)?;
            assert!(retained.ends_with(capture::MARKER));
            assert!(u64::try_from(retained.len())? <= artifacts::LIMIT);
            Ok(())
        })
}

#[test]
fn full_process_ledger_does_not_prevent_owned_term_reap() -> Result<()> {
    let root = tempfile::tempdir()?;
    let ledger = root.path().join("processes.jsonl");
    std::fs::File::create(&ledger)?.set_len(artifacts::LIMIT)?;
    let result = Process::spawn(
        root.path(),
        "ledger-overrun",
        Command::new("/usr/bin/sleep").arg("60"),
    );
    assert!(result.is_err());
    let evidence_path = std::fs::read_dir(root.path())?
        .collect::<std::io::Result<Vec<_>>>()?
        .into_iter()
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("process-") && name.ends_with(".json"))
        })
        .context("resource failure lacked independent child reap evidence")?;
    let retained = artifacts::json(&evidence_path)?;
    assert_eq!(retained.pointer("/evidence/reaped"), Some(&json!(true)));
    assert_eq!(retained.pointer("/evidence/exit/signal"), Some(&json!(15)));
    assert_eq!(
        retained.pointer("/evidence/requested_signal"),
        Some(&json!(15))
    );
    assert_eq!(std::fs::metadata(ledger)?.len(), artifacts::LIMIT);
    let pid = retained
        .pointer("/evidence/identity/pid")
        .and_then(Value::as_u64)
        .context("resource failure child PID absent")?;
    assert!(!Path::new(&format!("/proc/{pid}")).exists());
    Ok(())
}
