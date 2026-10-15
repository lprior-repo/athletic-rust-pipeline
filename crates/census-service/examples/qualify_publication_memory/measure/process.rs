use anyhow::{ensure, Result};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

#[path = "capture.rs"]
mod capture;
use capture::Capture;

#[cfg(unix)]
#[path = "signals.rs"]
mod signals;

pub const OUTPUT_LIMIT: usize = 1024 * 1024;
const CLEANUP: Duration = Duration::from_secs(2);
const POLL: Duration = Duration::from_millis(10);

pub struct Outcome {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub success: bool,
    pub code: Option<i32>,
    pub timed_out: bool,
    pub interruption: Option<Interruption>,
    pub output_exceeded: bool,
    pub cleanup_error: Option<String>,
    pub elapsed_seconds: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Interruption {
    Interrupt,
    Terminate,
}

enum Stop {
    Completed,
    Timeout,
    OutputLimit,
    Interrupted(Interruption),
}

#[cfg(unix)]
pub fn run(binary: &Path, root: &Path, out: &Path, timeout: Duration) -> Result<Outcome> {
    let mut signals = signals::Interrupts::register()?;
    let started = Instant::now();
    let (mut child, mut streams) = spawn(binary, root, out)?;
    let result = observe(&mut child, &mut streams, &mut signals, started, timeout);
    match result {
        Ok(outcome) => Ok(outcome),
        Err(primary) => match child.cleanup() {
            Ok(()) => Err(primary),
            Err(cleanup) => Err(anyhow::anyhow!("{primary:#}; cleanup failed: {cleanup:#}")),
        },
    }
}

#[cfg(unix)]
struct Streams {
    stdout: std::os::unix::net::UnixStream,
    stderr: std::os::unix::net::UnixStream,
    output: Capture,
    errors: Capture,
}

#[cfg(unix)]
fn spawn(binary: &Path, root: &Path, out: &Path) -> Result<(OwnedChild, Streams)> {
    use std::os::fd::OwnedFd;
    use std::os::unix::{net::UnixStream, process::CommandExt};
    ensure!(
        cfg!(target_os = "macos") || cfg!(target_os = "linux"),
        "only macOS/GNU time supported"
    );
    let (stdout, out_writer) = UnixStream::pair()?;
    let (stderr, err_writer) = UnixStream::pair()?;
    stdout.set_nonblocking(true)?;
    stderr.set_nonblocking(true)?;
    let mut command = Command::new("/usr/bin/time");
    command.arg(if cfg!(target_os = "macos") {
        "-l"
    } else {
        "-v"
    });
    command
        .env("LC_ALL", "C")
        .arg(binary)
        .arg("publish")
        .arg("--root")
        .arg(root)
        .arg("--out")
        .arg(out)
        .stdin(Stdio::null())
        .stdout(Stdio::from(OwnedFd::from(out_writer)))
        .stderr(Stdio::from(OwnedFd::from(err_writer)))
        .process_group(0);
    let child = OwnedChild {
        child: command.spawn()?,
        status: None,
        cleanup_attempted: false,
        cleanup_failure: None,
    };
    drop(command);
    Ok((
        child,
        Streams {
            stdout,
            stderr,
            output: Capture::default(),
            errors: Capture::default(),
        },
    ))
}

#[cfg(unix)]
fn observe(
    child: &mut OwnedChild,
    streams: &mut Streams,
    signals: &mut signals::Interrupts,
    started: Instant,
    timeout: Duration,
) -> Result<Outcome> {
    let stopped = streams.poll_until_exit(child, signals, started, timeout)?;
    let mut cleanup_error = child.cleanup().err().map(|error| format!("{error:#}"));
    if !streams.drain()? {
        let message = "output writers remain open after bounded drain; descendants may remain alive; no signal sent after direct-child reap";
        cleanup_error = Some(match cleanup_error {
            Some(error) => format!("{error}; {message}"),
            None => message.into(),
        });
    }
    Ok(Outcome {
        stdout: std::mem::take(&mut streams.output.bytes),
        stderr: std::mem::take(&mut streams.errors.bytes),
        success: matches!(stopped, Stop::Completed)
            && child.status.is_some_and(|status| status.success())
            && cleanup_error.is_none(),
        code: child.status.and_then(|status| status.code()),
        timed_out: matches!(stopped, Stop::Timeout),
        interruption: match stopped {
            Stop::Interrupted(signal) => Some(signal),
            _ => None,
        },
        output_exceeded: streams.output.exceeded || streams.errors.exceeded,
        cleanup_error,
        elapsed_seconds: started.elapsed().as_secs_f64(),
    })
}

#[cfg(unix)]
impl Streams {
    fn poll(&mut self) -> Result<()> {
        self.output.poll(&mut self.stdout)?;
        self.errors.poll(&mut self.stderr)
    }

    fn poll_until_exit(
        &mut self,
        child: &mut OwnedChild,
        signals: &mut signals::Interrupts,
        started: Instant,
        timeout: Duration,
    ) -> Result<Stop> {
        loop {
            self.poll()?;
            if child.poll()?.is_some() {
                return Ok(Stop::Completed);
            }
            if started.elapsed() >= timeout {
                return Ok(Stop::Timeout);
            }
            if self.output.exceeded || self.errors.exceeded {
                return Ok(Stop::OutputLimit);
            }
            if let Some(signal) = signals.wait(POLL)? {
                return Ok(Stop::Interrupted(signal));
            }
        }
    }

    fn drain(&mut self) -> Result<bool> {
        let started = Instant::now();
        while !(self.output.eof && self.errors.eof) && started.elapsed() < CLEANUP {
            self.poll()?;
            if !(self.output.eof && self.errors.eof) {
                std::thread::sleep(POLL);
            }
        }
        Ok(self.output.eof && self.errors.eof)
    }
}

#[cfg(not(unix))]
pub fn run(_: &Path, _: &Path, _: &Path, _: Duration) -> Result<Outcome> {
    anyhow::bail!("native publication measurement requires macOS or Linux")
}

struct OwnedChild {
    child: Child,
    status: Option<ExitStatus>,
    cleanup_attempted: bool,
    cleanup_failure: Option<String>,
}
impl OwnedChild {
    fn poll(&mut self) -> Result<Option<ExitStatus>> {
        if self.status.is_none() {
            self.status = self.child.try_wait()?;
        }
        Ok(self.status)
    }

    #[cfg(unix)]
    fn cleanup(&mut self) -> Result<()> {
        self.cleanup_with_after_signal(|| {})
    }

    #[cfg(unix)]
    fn cleanup_with_after_signal(&mut self, after_signal: impl FnOnce()) -> Result<()> {
        if !self.cleanup_attempted {
            self.cleanup_attempted = true;
            self.cleanup_failure = self
                .cleanup_once(after_signal)
                .err()
                .map(|error| format!("{error:#}"));
        }
        match &self.cleanup_failure {
            Some(error) => anyhow::bail!("{error}"),
            None => Ok(()),
        }
    }

    #[cfg(unix)]
    fn cleanup_once(&mut self, after_signal: impl FnOnce()) -> Result<()> {
        use rustix::process::{kill_process_group, Pid, Signal};
        if self.poll()?.is_some() {
            return Ok(());
        }
        let signal = kill_process_group(Pid::from_child(&self.child), Signal::TERM);
        after_signal();
        let started = Instant::now();
        let mut wait_error = None;
        loop {
            match self.poll() {
                Ok(Some(_)) => break,
                Ok(None) if started.elapsed() < CLEANUP => std::thread::sleep(POLL),
                Ok(None) => break,
                Err(error) => {
                    wait_error = Some(format!("wait failed: {error:#}"));
                    break;
                }
            }
        }
        let reap_error = self.status.is_none().then(|| format!(
            "child {} unreaped after bounded TERM cleanup; child/descendants may remain alive; no SIGKILL attempted", self.child.id()));
        let reap_error = match (reap_error, wait_error) {
            (Some(reap), Some(wait)) => Some(format!("{reap}; {wait}")),
            (reap, None) => reap,
            (None, wait) => wait,
        };
        match (signal, reap_error) {
            (Ok(()), None) => Ok(()),
            (Err(signal), None) => {
                anyhow::bail!("process-group TERM failed: {signal}; direct child reaped")
            }
            (Ok(()), Some(reap)) => anyhow::bail!("{reap}"),
            (Err(signal), Some(reap)) => {
                anyhow::bail!("process-group TERM failed: {signal}; {reap}")
            }
        }
    }

    #[cfg(not(unix))]
    fn cleanup(&mut self) -> Result<()> {
        anyhow::bail!("unsupported process cleanup platform")
    }
}
impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !self.cleanup_attempted {
            if let Err(error) = self.cleanup() {
                eprintln!("publication child cleanup failed: {error:#}");
            }
        }
    }
}

#[cfg(test)]
#[path = "process_tests.rs"]
mod tests;
