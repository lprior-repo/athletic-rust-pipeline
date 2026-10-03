use super::*;
use std::path::PathBuf;

const DRAIN: &str = "drained: accepted=3 completed=2 cancelled=1 timed_out=0 aborted=0 panicked=0";

#[test]
fn shutdown_schedule_fixture() -> Result<()> {
    let Some(root) = std::env::var_os("QUALIFICATION_SHUTDOWN_ROOT") else {
        return Ok(());
    };
    let root = PathBuf::from(root);
    let signal: i32 = std::env::var("QUALIFICATION_SHUTDOWN_SIGNAL")?.parse()?;
    let probe = std::env::var("QUALIFICATION_SHUTDOWN_PROBE")?;
    let failure = std::env::var("QUALIFICATION_SHUTDOWN_FAILURE")?;
    let mut signals = cancellation::Signals::install()?;
    let mut node = launch_child(&root, "node", &failure)?;
    let mut endpoint = launch_child(&root, "endpoint", &failure)?;
    let identity = json!({"boot_id":"isolated-boot","supervisor_pid":std::process::id(),"generation":process::generation()?,"endpoint":endpoint.identity(),"node":node.identity()});
    artifacts::publish(&root.join("current-owner.json"), &identity)?;
    if failure == "resource" {
        deliver(&root, signal, &signals)?;
    }
    let initial_reason = signals.reason();
    let waited = shutdown_step(
        || {
            scheduled_probe(
                &root,
                signal,
                &signals,
                probe == "endpoint" && failure != "resource",
                &mut endpoint,
            )
        },
        || {
            scheduled_probe(
                &root,
                signal,
                &signals,
                probe == "node" && failure != "resource",
                &mut node,
            )
        },
        || signals.reason(),
    )
    .map(|stopped| {
        assert_eq!(stopped, true);
    });
    artifacts::publish(
        &root.join("schedule.json"),
        &json!({"signal":signals.reason(),"initial_reason":initial_reason,"wait":format!("{waited:?}")}),
    )?;
    let result = finish_schedule(
        &root,
        &identity,
        &mut endpoint,
        &mut node,
        &waited,
        failure == "none",
    );
    signals.finish()?;
    result
}

fn finish_schedule(
    root: &Path,
    identity: &Value,
    endpoint: &mut Process,
    node: &mut Process,
    waited: &Result<()>,
    expected_success: bool,
) -> Result<()> {
    let endpoint_stop = endpoint.stop();
    let node_stop = node.stop();
    let certificate = certify(
        root,
        identity,
        endpoint,
        node,
        waited,
        &endpoint_stop,
        &node_stop,
    );
    if expected_success {
        waited
            .as_ref()
            .map_err(|error| anyhow::anyhow!("{error:#}"))?;
        endpoint_stop?;
        node_stop?;
        certificate?;
    } else {
        assert!(waited.is_err());
        assert!(certificate.is_err());
    }
    Ok(())
}

fn launch_child(root: &Path, label: &str, failure: &str) -> Result<Process> {
    let script = if failure == label {
        "exit 7".into()
    } else if label == "endpoint" && failure == "resource" {
        format!(
            "/usr/bin/head -c {} /dev/zero; exec /usr/bin/sleep 20",
            artifacts::LIMIT
                .checked_add(8192)
                .context("resource fixture size overflow")?
        )
    } else {
        let drain = if label == "endpoint" {
            DRAIN
        } else {
            "node stopped"
        };
        format!(
            "trap 'printf \"{drain}\\n\"; exit 0' TERM; printf 'ready\\n'; i=0; while [ \"$i\" -lt 2000 ]; do /usr/bin/sleep 0.01; i=$((i+1)); done; exit 9"
        )
    };
    let mut child = Process::spawn(root, label, Command::new("/bin/sh").args(["-c", &script]))?;
    if failure == label {
        assert_eq!(child.wait(100)?.code(), Some(7));
    } else {
        wait_ready(&mut child, failure == "resource" && label == "endpoint")?;
    }
    Ok(child)
}

fn wait_ready(child: &mut Process, resource: bool) -> Result<()> {
    (0..1000)
        .find_map(|_| {
            let ready = if resource {
                !matches!(child.health_status(), Ok(None))
            } else {
                match artifacts::read(&child.log) {
                    Ok(bytes) => bytes.starts_with(b"ready\n"),
                    Err(error) => return Some(Err(error)),
                }
            };
            if ready {
                return Some(Ok(()));
            }
            std::thread::sleep(Duration::from_millis(10));
            None
        })
        .context("shutdown child fixture did not become ready")?
}

fn scheduled_probe(
    root: &Path,
    signal: i32,
    signals: &cancellation::Signals,
    inject: bool,
    child: &mut Process,
) -> Result<Option<std::process::ExitStatus>> {
    if inject {
        assert_eq!(signals.reason(), 0);
        deliver(root, signal, signals)?;
    }
    child.health_status()
}

fn deliver(root: &Path, signal: i32, signals: &cancellation::Signals) -> Result<()> {
    cancellation::cleanup(|| {
        process::command(
            root,
            "schedule-signal",
            Command::new("/usr/bin/kill").args([
                "-s",
                &signal.to_string(),
                &std::process::id().to_string(),
            ]),
            100,
        )
    })?;
    (0..1000)
        .find_map(|_| {
            if signals.reason() == signal {
                return Some(Ok(()));
            }
            std::thread::sleep(Duration::from_millis(10));
            None
        })
        .context("signal worker did not publish scheduled TERM/INT")?
}
