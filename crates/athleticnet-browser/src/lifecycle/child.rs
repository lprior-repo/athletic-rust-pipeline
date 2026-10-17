use super::error::BrowserStartupError;
use crate::SHUTDOWN_TIMEOUT;
use tokio::io::AsyncReadExt;
use tokio::process::{Child, ChildStderr, Command};
use tokio::time::Instant;

const MAX_STDERR_BYTES: usize = 65_536;
const MAX_STDERR_LINE: usize = 4096;

pub(crate) struct OwnedChild {
    child: Child,
    discovery: EndpointReader,
}

struct EndpointReader {
    stderr: Option<ChildStderr>,
    line: [u8; MAX_STDERR_LINE],
    length: usize,
    visited: usize,
    deadline: Instant,
}

impl OwnedChild {
    pub(super) fn launch(command: &mut Command) -> Result<Self, BrowserStartupError> {
        let deadline = Instant::now()
            .checked_add(SHUTDOWN_TIMEOUT)
            .ok_or_else(|| launch_failure("endpoint deadline overflow"))?;
        let mut child = command.spawn().map_err(launch_error)?;
        let stderr = child.stderr.take();
        Ok(Self {
            child,
            discovery: EndpointReader {
                stderr,
                line: [0; MAX_STDERR_LINE],
                length: 0,
                visited: 0,
                deadline,
            },
        })
    }

    pub(super) async fn endpoint(&mut self) -> Result<url::Url, BrowserStartupError> {
        let endpoint = tokio::time::timeout_at(self.discovery.deadline, self.discovery.read())
            .await
            .map_err(|_| launch_failure("owned browser endpoint discovery timed out"))??;
        drop(self.discovery.stderr.take());
        Ok(endpoint)
    }

    pub(crate) async fn terminate_and_wait(&mut self) -> Result<(), BrowserStartupError> {
        if self.child.try_wait().map_err(cleanup_error)?.is_some() {
            return Ok(());
        }
        terminate(&self.child)?;
        self.wait().await
    }

    pub(crate) async fn wait(&mut self) -> Result<(), BrowserStartupError> {
        tokio::time::timeout(SHUTDOWN_TIMEOUT, self.child.wait())
            .await
            .map_err(|_| cleanup_failure("owned browser wait timed out"))?
            .map_err(cleanup_error)?;
        Ok(())
    }
}

impl EndpointReader {
    async fn read(&mut self) -> Result<url::Url, BrowserStartupError> {
        let mut chunk = [0u8; 1024];
        while self.visited < MAX_STDERR_BYTES {
            let stderr = self
                .stderr
                .as_mut()
                .ok_or_else(|| launch_failure("owned browser stderr missing"))?;
            let count = stderr.read(&mut chunk).await.map_err(launch_error)?;
            if count == 0 {
                return Err(launch_failure("owned browser stderr ended before endpoint"));
            }
            for byte in chunk
                .get(..count)
                .ok_or_else(|| launch_failure("invalid stderr read size"))?
            {
                self.visited = self.visited.saturating_add(1);
                if self.visited > MAX_STDERR_BYTES {
                    return Err(launch_failure("owned browser stderr byte limit"));
                }
                if let Some(endpoint) = self.push(*byte)? {
                    return Ok(endpoint);
                }
            }
        }
        Err(launch_failure("owned browser stderr byte limit"))
    }

    fn push(&mut self, byte: u8) -> Result<Option<url::Url>, BrowserStartupError> {
        if byte == b'\n' {
            let line = self
                .line
                .get(..self.length)
                .ok_or_else(|| launch_failure("stderr line bounds"))?;
            let endpoint = parse_endpoint(line)?;
            self.length = 0;
            return Ok(endpoint);
        }
        let slot = self
            .line
            .get_mut(self.length)
            .ok_or_else(|| launch_failure("owned browser stderr line limit"))?;
        *slot = byte;
        self.length = self.length.saturating_add(1);
        Ok(None)
    }
}

fn parse_endpoint(line: &[u8]) -> Result<Option<url::Url>, BrowserStartupError> {
    let Some(raw) = line.strip_prefix(b"DevTools listening on ") else {
        return Ok(None);
    };
    let text = std::str::from_utf8(raw).map_err(|_| launch_failure("invalid endpoint encoding"))?;
    let endpoint = url::Url::parse(text.trim_end_matches('\r'))
        .map_err(|_| launch_failure("invalid endpoint URL"))?;
    if endpoint.scheme() != "ws"
        || !matches!(
            endpoint.host(),
            Some(url::Host::Ipv4(std::net::Ipv4Addr::LOCALHOST))
                | Some(url::Host::Ipv6(std::net::Ipv6Addr::LOCALHOST))
        )
        || endpoint.port().is_none()
        || !endpoint.path().starts_with("/devtools/browser/")
        || !endpoint.username().is_empty()
        || endpoint.password().is_some()
        || endpoint.query().is_some()
        || endpoint.fragment().is_some()
    {
        return Err(launch_failure("owned browser announced a foreign endpoint"));
    }
    Ok(Some(endpoint))
}

#[cfg(unix)]
fn terminate(child: &Child) -> Result<(), BrowserStartupError> {
    use rustix::process::{kill_process, Pid, Signal};
    let raw = child
        .id()
        .ok_or_else(|| cleanup_failure("owned browser PID unavailable"))?;
    let pid = Pid::from_raw(
        i32::try_from(raw).map_err(|_| cleanup_failure("owned browser PID overflow"))?,
    )
    .ok_or_else(|| cleanup_failure("owned browser PID invalid"))?;
    kill_process(pid, Signal::TERM).map_err(|error| cleanup_error(error.into()))
}

#[cfg(not(unix))]
fn terminate(_child: &Child) -> Result<(), BrowserStartupError> {
    Err(cleanup_failure(
        "owned browser SIGTERM is unavailable on this platform",
    ))
}

fn launch_failure(message: &str) -> BrowserStartupError {
    BrowserStartupError::LaunchFailed(message.to_owned())
}

fn launch_error(error: std::io::Error) -> BrowserStartupError {
    launch_failure(&error.to_string())
}

fn cleanup_failure(message: &str) -> BrowserStartupError {
    BrowserStartupError::ShutdownFailed(message.to_owned())
}

fn cleanup_error(error: std::io::Error) -> BrowserStartupError {
    cleanup_failure(&error.to_string())
}

#[cfg(test)]
mod tests;
