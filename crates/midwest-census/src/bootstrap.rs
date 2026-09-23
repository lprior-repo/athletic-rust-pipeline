//! Service supervisor: an owned task region with a cancel/drain/finalize shutdown protocol.
//!
//! The batch CLI opens the store, does one pass, and exits. The service path has to survive
//! operators, restarts, and Restate journal replays, so it is built as an explicit region: [`serve()`]
//! (or [`serve_until`]) owns exactly one task — the HTTP endpoint — waits for a stop request, drains
//! inside a bounded deadline, finalizes the store, and reports what happened. A bare `tokio::spawn`
//! whose handle is dropped would be an orphan factory; there is none here.
//!
//! Shutdown is a protocol, not a flag:
//!
//! 1. **request** — a signal, or the caller's `shutdown` future, resolves and the reason is recorded;
//! 2. **stop intake** — `HttpServer::serve_with_cancel` stops accepting and runs a hyper graceful
//!    shutdown, so in-flight invocations get to finish;
//! 3. **drain** — the region is reaped inside `drain_timeout`; whatever outlives the deadline is
//!    aborted and counted, never leaked;
//! 4. **finalize** — the store is flushed (`SyncAll`) and the Fjall database dropped, so the next
//!    start replays only what reached the journal;
//! 5. **report** — [`DrainReport`] counts accepted/completed/cancelled/timed out/aborted/panicked work
//!    plus the [`StopReason`], which is what tests and operators assert on.
//!
//! Time enters the application through the [`Clock`] capability rather than `SystemTime::now` inside
//! domain logic, so a replayed workflow can run against a fixed date.
//!
//! # Errors
//!
//! Every stage below — flag parsing, the store, the bind, drain accounting — reports
//! [`BootstrapError`]. `anyhow` survives in exactly one place: the [`serve()`] and [`serve_until`]
//! signatures, which `midwest-serve` prints with its chain and the integration test drives as
//! `JoinSet<anyhow::Result<DrainReport>>`. Converting those two would push a type change into
//! `bin/**` and `tests/**`, so the conversion happens once, at the boundary.

use std::time::Duration;

mod drain;
mod error;
mod guard;
mod options;
mod serve;
mod stop;

pub use error::BootstrapError;

/// Default fan-out cap for concurrent work started by the services.
pub const DEFAULT_MAX_CONCURRENT: usize = 8;
/// Default deadline for in-flight work after a stop request.
pub const DEFAULT_DRAIN_TIMEOUT: Duration = Duration::from_secs(30);
/// Default process memory ceiling: past this the region drains and exits rather than swap the
/// machine out from under its operator. It bounds the *process*, not the run — Restate replays
/// whatever the drain left unfinished, so the next start resumes where this stopped.
pub const DEFAULT_MEMORY_BUDGET_BYTES: u64 = 48 * 1024 * 1024 * 1024;

pub use crate::clock::{Clock, SystemClock};

pub use drain::DrainReport;
pub use options::ServeOptions;
pub use serve::{init_tracing, serve, serve_until};
pub use stop::StopReason;

// The usage text stays in the facade because `error` prints it from three flag messages through
// `super::USAGE`; the parser that produces those flags lives in `options`.
const USAGE: &str = "midwest-serve [--listen ADDR] [--data-dir DIR] \
                     [--max-concurrent N] [--drain-timeout SECONDS] [--help]";

// The verbatim `tests` module resolves the supervisor, the drain entry point and the two tokio
// types through `use super::*`, exactly as the net split feeds its tests module from `mod.rs`.
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
