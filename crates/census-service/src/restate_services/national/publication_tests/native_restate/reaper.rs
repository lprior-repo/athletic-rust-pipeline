use std::path::{Path, PathBuf};
use std::process::{Child, ExitStatus};
use std::thread;
use std::time::{Duration, Instant};
use tempfile::TempDir;

const REAPER_TIMEOUT: Duration = Duration::from_secs(35);
const REAPER_POLL: Duration = Duration::from_millis(100);

#[cfg(unix)]
pub(super) fn request_sigterm(child: &Child) -> std::io::Result<()> {
    rustix::process::kill_process(
        rustix::process::Pid::from_child(child),
        rustix::process::Signal::TERM,
    )
    .map_err(std::io::Error::from)
}

#[cfg(not(unix))]
pub(super) fn request_sigterm(_child: &Child) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "native Restate fixture requires Unix SIGTERM",
    ))
}

pub(super) fn handoff(
    child: Child,
    mut root: TempDir,
    reason: &str,
    cleanup_on_exit: bool,
) -> String {
    let pid = child.id();
    let path = root.path().to_path_buf();
    root.disable_cleanup(true);
    let message = format!(
        "{reason}; cleanup handoff pid={pid} path={} reaper=native-restate-reaper",
        path.display()
    );
    let thread_message = message.clone();
    match thread::Builder::new()
        .name(String::from("native-restate-reaper"))
        .spawn(move || reap(child, root, path, thread_message, cleanup_on_exit))
    {
        Ok(_) => message,
        Err(error) => format!("{message}; reaper thread spawn failed: {error}; root retained"),
    }
}

pub(super) fn cleanup_stopped(mut root: TempDir) {
    let path = root.path().to_path_buf();
    root.disable_cleanup(true);
    let message = format!(
        "native Restate stopped node cleanup pid=none path={}",
        path.display()
    );
    let thread_message = message.clone();
    if let Err(error) = thread::Builder::new()
        .name(String::from("native-restate-reaper"))
        .spawn(move || remove_stopped_root(root, &path, &thread_message))
    {
        eprintln!("{message}; reaper thread spawn failed: {error}; root retained");
    }
}

fn reap(mut child: Child, root: TempDir, path: PathBuf, handoff: String, cleanup_on_exit: bool) {
    match child.try_wait() {
        Ok(Some(status)) => {
            finish(root, &path, &handoff, status, cleanup_on_exit);
            return;
        }
        Ok(None) => {}
        Err(error) => {
            drop(root);
            eprintln!(
                "{handoff}; reaper could not inspect child before stop: {error}; root retained"
            );
            return;
        }
    }
    let cleanup_on_exit = match request_sigterm(&child) {
        Ok(()) => cleanup_on_exit,
        Err(error) => {
            eprintln!("{handoff}; reaper could not request graceful stop: {error}; root retained");
            false
        }
    };
    let deadline = Instant::now() + REAPER_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                finish(root, &path, &handoff, status, cleanup_on_exit);
                return;
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(REAPER_POLL),
            Ok(None) => {
                drop(root);
                eprintln!("{handoff}; reaper grace period expired; root retained");
                return;
            }
            Err(error) => {
                drop(root);
                eprintln!("{handoff}; reaper could not inspect child: {error}; root retained");
                return;
            }
        }
    }
}

fn finish(root: TempDir, path: &Path, handoff: &str, status: ExitStatus, cleanup_on_exit: bool) {
    if !status.success() {
        drop(root);
        eprintln!("{handoff}; reaper observed unsuccessful exit: {status}; root retained");
        return;
    }
    if !cleanup_on_exit {
        drop(root);
        eprintln!("{handoff}; stopped child root retained by cleanup policy");
        return;
    }
    remove_stopped_root(root, path, handoff);
}

fn remove_stopped_root(root: TempDir, path: &Path, handoff: &str) {
    match std::fs::remove_dir_all(path) {
        Ok(()) => drop(root),
        Err(error) => {
            drop(root);
            eprintln!(
                "{handoff}; reaper could not remove stopped node root: {error}; root retained"
            );
        }
    }
}
