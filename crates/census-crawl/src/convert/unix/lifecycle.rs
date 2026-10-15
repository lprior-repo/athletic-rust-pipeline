use super::super::{converter_io, ConverterLifecycleError, ConverterObligation};
use super::{combine, CONVERTER_POLL};
use crate::{CrawlError, CrawlResult};
use rustix::io::Errno;
use rustix::process::{kill_process_group, test_kill_process_group, Pid, Signal};
use std::process::{Child, ExitStatus};

const CLEANUP_STEPS: usize = 200;

pub(super) struct Termination {
    status: CrawlResult<ExitStatus>,
    obligations: CrawlResult<()>,
}

impl Termination {
    pub(super) fn finish<T>(self, command: &str, primary: CrawlResult<T>) -> CrawlResult<T> {
        match primary {
            Ok(value) => {
                let outcome = self.status.and_then(|status| {
                    if status.success() {
                        Ok(value)
                    } else {
                        Err(converter_io(
                            command,
                            std::io::Error::other(format!("{command} exited with {status}")),
                        ))
                    }
                });
                combine(outcome, self.obligations)
            }
            Err(error) => combine(Err(error), combine(self.status.map(drop), self.obligations)),
        }
    }
}

pub(super) fn terminate(child: &mut Child, command: &str) -> Termination {
    let pid = Pid::from_child(child);
    let signal = match super::peek(pid) {
        Ok(_) => match kill_process_group(pid, Signal::KILL) {
            Ok(()) | Err(Errno::SRCH) => Ok(()),
            Err(error) => Err(unproven(
                command,
                ConverterObligation::GroupSignal,
                error.into(),
            )),
        },
        Err(error) => Err(unproven(
            command,
            ConverterObligation::LeaderIdentity,
            error.into(),
        )),
    };
    let status = reap(child, command);
    let absent = group_absent(pid, command);
    Termination {
        status,
        obligations: combine(signal, absent),
    }
}

fn reap(child: &mut Child, command: &str) -> CrawlResult<ExitStatus> {
    for _ in 0..CLEANUP_STEPS {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) => std::thread::sleep(CONVERTER_POLL),
            Err(error) => {
                return Err(unproven(command, ConverterObligation::LeaderReaped, error));
            }
        }
    }
    Err(unproven(
        command,
        ConverterObligation::LeaderReaped,
        std::io::Error::other("leader remained unreaped after the cleanup budget"),
    ))
}

fn group_absent(pid: Pid, command: &str) -> CrawlResult<()> {
    for _ in 0..CLEANUP_STEPS {
        match test_kill_process_group(pid) {
            Err(Errno::SRCH) => return Ok(()),
            Ok(()) => std::thread::sleep(CONVERTER_POLL),
            Err(error) => {
                return Err(unproven(
                    command,
                    ConverterObligation::GroupAbsent,
                    error.into(),
                ));
            }
        }
    }
    Err(unproven(
        command,
        ConverterObligation::GroupAbsent,
        std::io::Error::other("group remains observable; signal delivery is not reaping proof"),
    ))
}

pub(super) fn unproven(
    command: &str,
    obligation: ConverterObligation,
    source: std::io::Error,
) -> CrawlError {
    ConverterLifecycleError::Unproven {
        command: command.to_owned(),
        obligation,
        source,
    }
    .into()
}

#[cfg(test)]
#[path = "lifecycle/tests.rs"]
mod tests;
