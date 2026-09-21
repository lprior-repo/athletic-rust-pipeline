//! The `midwest-serve` command line: the parsed surface and the parser that refuses typos.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use super::error::BootstrapError;
use super::{DEFAULT_DRAIN_TIMEOUT, DEFAULT_MAX_CONCURRENT};

#[derive(Debug, Clone)]
pub struct ServeOptions {
    pub listen: SocketAddr,
    pub data_dir: PathBuf,
    pub max_concurrent: usize,
    pub drain_timeout: Duration,
}

impl Default for ServeOptions {
    fn default() -> Self {
        Self {
            listen: SocketAddr::from(([127, 0, 0, 1], 9080)),
            data_dir: PathBuf::from("var/midwest-census"),
            max_concurrent: DEFAULT_MAX_CONCURRENT,
            drain_timeout: DEFAULT_DRAIN_TIMEOUT,
        }
    }
}

impl ServeOptions {
    /// Parse `--flag value` pairs. Unknown flags are rejected instead of ignored: a typo in a
    /// deployment script must not silently serve the wrong directory.
    pub fn from_env(args: impl Iterator<Item = String>) -> Result<Self, BootstrapError> {
        let mut options = ServeOptions::default();
        let mut args = args.peekable();
        while let Some(flag) = args.next() {
            let mut value = |flag: &str| -> Result<String, BootstrapError> {
                args.next().ok_or_else(|| BootstrapError::MissingValue {
                    flag: flag.to_string(),
                })
            };
            match flag.as_str() {
                "--listen" => {
                    let raw = value("--listen")?;
                    options.listen = raw
                        .parse()
                        .map_err(|source| BootstrapError::ListenNotAnAddress { raw, source })?;
                }
                "--data-dir" => options.data_dir = PathBuf::from(value("--data-dir")?),
                "--max-concurrent" => {
                    let raw = value("--max-concurrent")?;
                    let parsed: usize = raw
                        .parse()
                        .map_err(|source| BootstrapError::ConcurrencyNotANumber { raw, source })?;
                    if parsed == 0 {
                        return Err(BootstrapError::ConcurrencyIsZero);
                    }
                    options.max_concurrent = parsed;
                }
                "--drain-timeout" => {
                    let raw = value("--drain-timeout")?;
                    let seconds: u64 =
                        raw.parse()
                            .map_err(|source| BootstrapError::DrainTimeoutNotASeconds {
                                raw,
                                source,
                            })?;
                    options.drain_timeout = Duration::from_secs(seconds);
                }
                "--help" | "-h" => return Err(BootstrapError::HelpRequested),
                other => {
                    return Err(BootstrapError::UnknownFlag {
                        flag: other.to_string(),
                    })
                }
            }
        }
        Ok(options)
    }
}
