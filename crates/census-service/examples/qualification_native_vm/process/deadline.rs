use super::*;
use std::fmt;
use std::time::Instant;

#[derive(Debug)]
pub(crate) struct DeadlineExpired;

impl fmt::Display for DeadlineExpired {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("owned observation monotonic deadline expired")
    }
}

impl std::error::Error for DeadlineExpired {}

fn remaining(deadline: Instant) -> Result<Duration> {
    deadline
        .checked_duration_since(Instant::now())
        .filter(|duration| !duration.is_zero())
        .ok_or_else(|| DeadlineExpired.into())
}

impl Process {
    fn wait_until(&mut self, deadline: Instant) -> Result<ExitStatus> {
        ensure!(
            remaining(deadline)? <= Duration::from_secs(3600),
            "child observation exceeds one-hour fixed budget"
        );
        (0..36_000)
            .find_map(|_| {
                let observed = (|| -> Result<Option<ExitStatus>> {
                    remaining(deadline)?;
                    cancellation::checkpoint()?;
                    self.capture.check()?;
                    if let Some(status) = self.child.try_wait()? {
                        self.status = Some(status);
                        remaining(deadline)?;
                        return Ok(Some(status));
                    }
                    std::thread::sleep(remaining(deadline)?.min(Duration::from_millis(100)));
                    Ok(None)
                })();
                match observed {
                    Ok(Some(status)) => Some(Ok(status)),
                    Ok(None) => None,
                    Err(error) => Some(Err(error)),
                }
            })
            .context("owned observation exhausted fixed polling budget")?
    }
}

pub(crate) fn command_until(
    root: &Path,
    label: &str,
    command: &mut Command,
    deadline: Instant,
) -> Result<String> {
    remaining(deadline)?;
    let mut process = Process::spawn(root, label, command)?;
    let waited = process.wait_until(deadline);
    let finalized = if process.status.is_some() {
        process.finish_reap()
    } else {
        Ok(())
    };
    let completed = match waited {
        Ok(status) => finalized.and_then(|()| {
            remaining(deadline)?;
            let output = String::from_utf8(artifacts::read(&process.log)?)?;
            remaining(deadline)?;
            ensure!(status.success(), "{label} failed: {status}: {output}");
            Ok(output)
        }),
        Err(error) => Err(error.context(format!("capture/reap finalization={finalized:?}"))),
    };
    match completed {
        Ok(output) => Ok(output),
        Err(error) => {
            let cleanup = process.stop();
            Err(error.context(format!(
                "{label} observation failed; separate bounded cleanup={cleanup:?}; evidence={}",
                process.evidence()
            )))
        }
    }
}
