use std::time::Duration;

mod drain;
mod error;
mod guard;
mod options;
mod serve;
mod stop;
mod store;

pub use error::BootstrapError;

pub const DEFAULT_MAX_CONCURRENT: usize =
    census_domain::model::ServiceResourceProfileV1::ENROLLMENT_DEFAULTS_V1.max_concurrent();
pub const MAX_CONCURRENT_CEILING: usize = 1024;
pub const DEFAULT_DRAIN_TIMEOUT: Duration = Duration::from_secs(
    census_domain::model::ServiceResourceProfileV1::ENROLLMENT_DEFAULTS_V1.drain_timeout_seconds(),
);
pub const DEFAULT_MEMORY_BUDGET_BYTES: u64 =
    census_domain::model::ServiceResourceProfileV1::ENROLLMENT_DEFAULTS_V1.memory_budget_bytes();
pub const MEMORY_BUDGET_CEILING_GIB: u64 = 1024;

pub use census_store::clock::{Clock, SystemClock};

pub use drain::{DrainReport, EndpointShutdown};
pub use options::{MemoryAccounting, ServeCaptureProtocol, ServeOptions};
pub use serve::{init_tracing, serve, serve_until};
pub use stop::StopReason;

const USAGE: &str = "census-serve [--listen ADDR] [--data-dir DIR] \
                     [--capture-protocol fresh-qualification-v3] \
                     [--max-concurrent N] [--drain-timeout SECONDS] [--memory-budget-gib GIB] \
                     [--memory-cgroup HIERARCHY_PATH] \
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
