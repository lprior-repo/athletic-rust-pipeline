use std::time::Duration;

mod drain;
mod error;
mod guard;
mod options;
mod serve;
mod stop;

pub use error::BootstrapError;

pub const DEFAULT_MAX_CONCURRENT: usize = 8;
pub const MAX_CONCURRENT_CEILING: usize = 1024;
pub const DEFAULT_DRAIN_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_MEMORY_BUDGET_BYTES: u64 = 48 * 1024 * 1024 * 1024;

pub use census_store::clock::{Clock, SystemClock};

pub use drain::{DrainReport, EndpointShutdown};
pub use options::ServeOptions;
pub use serve::{init_tracing, serve, serve_until};
pub use stop::StopReason;

const USAGE: &str = "census-serve [--listen ADDR] [--data-dir DIR] \
                     [--max-concurrent N] [--drain-timeout SECONDS] \
                     [--browser-profile DIR] [--browser-executable PATH] [--browser-headless] [--help]";

#[cfg(test)]
use drain::drain;
#[cfg(test)]
use serve::supervise;
#[cfg(test)]
use std::net::SocketAddr;
#[cfg(test)]
use tokio::task::JoinSet;

#[cfg(test)]
mod tests;
