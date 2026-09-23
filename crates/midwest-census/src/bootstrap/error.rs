//! Typed failures of the service shell.
//!
//! Each variant names the stage that failed - the operator's flags, the endpoint's bind, the store
//! stages the supervisor owns, drain accounting - so a store failure keeps its own [`StoreError`]
//! rather than being re-described here.

use std::net::SocketAddr;
use std::path::PathBuf;

use crate::outcome::DrainState;
use census_store::StoreError;

use super::USAGE;

/// Failures of the service shell: the operator's flags, the endpoint's bind, the store stages the
/// supervisor owns, and drain accounting. A store failure keeps its own [`StoreError`], so this enum
/// names the *stage* that failed instead of re-describing the database.
#[derive(Debug, thiserror::Error)]
pub enum BootstrapError {
    /// A flag that takes a value was the last argument.
    #[error("{flag} needs a value\n{}", USAGE)]
    MissingValue { flag: String },
    /// `--listen` was not a socket address.
    #[error("{raw} is not a socket address")]
    ListenNotAnAddress {
        raw: String,
        #[source]
        source: std::net::AddrParseError,
    },
    /// `--max-concurrent` was not a positive integer.
    #[error("{raw} is not a positive integer")]
    ConcurrencyNotANumber {
        raw: String,
        #[source]
        source: std::num::ParseIntError,
    },
    /// `--max-concurrent 0` would refuse every job the service admits.
    #[error("--max-concurrent must be at least 1")]
    ConcurrencyIsZero,
    /// `--drain-timeout` was not a whole number of seconds.
    #[error("{raw} is not a number of seconds")]
    DrainTimeoutNotASeconds {
        raw: String,
        #[source]
        source: std::num::ParseIntError,
    },
    /// `--help`: the usage text *is* the message.
    #[error("{}", USAGE)]
    HelpRequested,
    /// A flag this build does not know.
    #[error("unknown flag {flag}\n{}", USAGE)]
    UnknownFlag { flag: String },
    /// `--listen` was not a loopback address, and the endpoint carries no identity key.
    #[error(
        "--listen {listen} is not a loopback address: the endpoint has no identity key configured, \
         so it must not be reachable from another host"
    )]
    NonLoopbackListen { listen: SocketAddr },
    /// The endpoint could not bind its address.
    #[error("binding {listen} failed: {source}")]
    Bind {
        listen: SocketAddr,
        #[source]
        source: std::io::Error,
    },
    /// The bound address could not be read back.
    #[error("reading the bound address failed: {source}")]
    BoundAddress {
        #[source]
        source: std::io::Error,
    },
    /// The store directory could not be created.
    #[error("creating {path} failed: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// The store could not be opened.
    #[error("opening the store under {path} failed: {source}")]
    StoreOpen {
        path: PathBuf,
        #[source]
        source: StoreError,
    },
    /// The store bootstrap task panicked or was cancelled.
    #[error("joining the store bootstrap task failed: {reason}")]
    StoreTask { state: DrainState, reason: String },
    /// The store could not be flushed on the way out.
    #[error("persisting the store during shutdown failed: {source}")]
    StoreFlush {
        #[source]
        source: StoreError,
    },
    /// A shutdown signal subscription could not be installed.
    #[error("subscribing to {signal} failed: {source}")]
    Signal {
        signal: &'static str,
        #[source]
        source: std::io::Error,
    },
    /// A drain counter did not fit its report field.
    #[error("task count does not fit u64")]
    TaskCountOverflow,
    /// `--browser-profile` enables the headed lane, whose origin - the source its profile reads -
    /// is compiled in; a build that cannot parse it cannot serve the lane.
    #[error("the browser lane's origin {origin} is not a URL: {source}")]
    LaneOriginUnusable {
        origin: String,
        source: url::ParseError,
    },
    /// `--browser-executable` or `--browser-headless` was given without `--browser-profile`, which
    /// is what enables the lane: a deployment that configures a lane it does not serve has a typo.
    #[error("{flag} configures the browser lane, which --browser-profile enables")]
    LaneFlagWithoutProfile { flag: String },
    /// The lane needs an absolute browser. Neither `--browser-executable` nor a `PATH` entry named
    /// one: the default is a program name, and a program name is only meaningful against `PATH`.
    #[error("{program} is on no PATH entry: name the browser with --browser-executable")]
    LaneBrowserNotOnPath { program: String },
    /// The census's client for the deployment's browser lane could not be built. The origin is this
    /// build's own constant, so the failure is a build or environment fault rather than a source's:
    /// an endpoint whose services cannot reach the lane cannot sweep a browser-transported source.
    #[error("building the browser lane's client for {origin} failed: {detail}")]
    LaneIngressUnusable { origin: String, detail: String },
}
