use super::super::{
    artifacts,
    process::{command, OwnedProcess},
};
use super::*;
use serde_json::json;
use std::path::Path;
use std::sync::{atomic::AtomicBool, Arc};
use std::time::Duration;

const CHILD_ROOT: &str = "CENSUS_QUALIFICATION_SIGNAL_TEST_ROOT";
const TEST: &str = "qualification_native_teams::signals::tests::driver_signals_preserve_owned_child_until_explicit_term_and_reap";

#[test]
fn driver_signals_preserve_owned_child_until_explicit_term_and_reap() -> Result<()> {
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        return child_region(Path::new(&root));
    }
    ["-TERM", "-INT"]
        .into_iter()
        .try_for_each(|signal| -> Result<()> {
            let root = tempfile::tempdir()?;
            let root_text = root.path().to_str().context("test root is not UTF-8")?;
            let mut driver = OwnedProcess::spawn(
                root.path(),
                "driver",
                &std::env::current_exe()?,
                &["--exact".into(), TEST.into(), "--nocapture".into()],
                &[(CHILD_ROOT, root_text)],
                Arc::new(AtomicBool::new(false)),
            )?;
            (0..500)
                .find_map(|_| {
                    if root.path().join("owned-ready.json").exists() {
                        return Some(Ok(()));
                    }
                    match driver.ensure_running() {
                        Ok(()) => {
                            std::thread::sleep(Duration::from_millis(10));
                            None
                        }
                        Err(error) => Some(Err(error)),
                    }
                })
                .context("isolated signal region did not become ready")??;
            command(
                root.path(),
                "driver-signal",
                Path::new("/usr/bin/kill"),
                &[signal.into(), driver.pid().to_string()],
            )?;
            assert_eq!(driver.wait(100)?.code(), Some(0));
            let observed: serde_json::Value = serde_json::from_str(&artifacts::read_bounded(
                &root.path().join("signal-owned-reap.json"),
            )?)?;
            assert_eq!(observed.get("child_reaped"), Some(&json!(true)));
            assert_eq!(observed.get("child_success"), Some(&json!(false)));
            assert_eq!(observed.get("exit_signal"), Some(&json!(15)));
            assert_eq!(driver.stop()?.get("success"), Some(&json!(true)));
            Ok(())
        })
}

fn child_region(root: &Path) -> Result<()> {
    use std::os::unix::process::ExitStatusExt;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let mut signals = {
        let _entered = runtime.enter();
        ShutdownSignals::install()?
    };
    let mut owned = OwnedProcess::spawn(
        root,
        "owned-sleeper",
        Path::new("/usr/bin/sleep"),
        &["30".into()],
        &[],
        Arc::new(AtomicBool::new(false)),
    )?;
    artifacts::write_json(
        &root.join("owned-ready.json"),
        &json!({"pid":std::process::id()}),
    )?;
    let requested = runtime.block_on(async {
        tokio::select! {
            result = signals.requested() => result,
            () = tokio::time::sleep(Duration::from_secs(5)) => anyhow::bail!("driver signal not observed"),
        }
    });
    let cleanup = owned.stop();
    requested?;
    let evidence = cleanup?;
    let status = owned
        .reaped_status()
        .context("owned child not reaped after signal")?;
    artifacts::write_json(
        &root.join("signal-owned-reap.json"),
        &json!({"child_reaped":evidence.get("reaped"),"child_success":evidence.get("success"),"exit_signal":status.signal()}),
    )?;
    Ok(())
}
