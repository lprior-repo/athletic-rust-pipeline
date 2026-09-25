//! The ceilings one invocation is bounded by: rows in an ingest request, windows and endpoints in a
//! sweep, rosters one jurisdiction walks in a run, and the two timers that bound how long any of
//! that work may stay in flight.
//!
//! Every one of them is admission control rather than a nicety of the code that reads it — the bound
//! is what keeps an invocation's memory and journal entry predictable, and what makes the durable
//! loops finite by construction. A caller may always ask for less; a request over a ceiling is
//! terminal, because replaying it would fail identically. The timers are the one ceiling Restate
//! reads from the endpoint manifest rather than from the handler, so they are declared on the
//! service definitions instead of being checked inside one.

use std::time::Duration;

use restate_sdk::prelude::*;

/// Ceiling on rows in one ingest request. Callers split larger batches: the bound keeps one
/// invocation's memory and journal entry predictable.
pub const MAX_ROWS_PER_REQUEST: usize = 50_000;
/// Ceiling on windows one sweep may observe, so the durable loop is bounded by construction.
pub const MAX_SWEEP_WINDOWS: u32 = 366;

/// Ceiling on the endpoints one sweep may observe. Each endpoint costs a durable object call, so an
/// unbounded list is an admission-control hole; a larger fleet is observed by successive sweeps.
pub const MAX_SWEEP_ENDPOINTS: usize = 256;

/// Ceiling on the rosters one jurisdiction's object walks in one run. The ceiling is admission
/// control like the sweeps': a caller may ask for fewer, and a state with more teams than this is
/// covered by successive runs rather than by one unbounded invocation.
pub const MAX_LIMIT_PER_STATE: usize = 10_000;

/// How long an invocation on this endpoint may stay in flight without journal progress.
///
/// Every handler here runs its work on the blocking pool, and the fan-out submits far more
/// jurisdiction handlers than the pool has slots, so waiting for a slot is part of an invocation's
/// in-flight time and produces no journal entry at all. Restate's one-minute default read that wait as
/// a stalled handler: on 2026-09-24 it asked 47 of the nationwide run's 49 jurisdiction invocations to
/// suspend, and aborted each one ten minutes later while it was still queued. A census handler is
/// allowed an hour of queueing; the fetch layer's own timeouts bound any single request.
const CENSUS_INACTIVITY_TIMEOUT: Duration = Duration::from_secs(60 * 60);

/// How long Restate waits for an invocation to react after asking it to suspend.
///
/// The work a handler does is not journaled until it lands, so an abort mid-job discards it and the
/// invocation's next attempt replays it. This is the backstop for a handler that is genuinely stuck
/// rather than queued: an hour is longer than any one jurisdiction's walk, and the invocation's own
/// retry policy is what decides what happens after an abort.
const CENSUS_ABORT_TIMEOUT: Duration = Duration::from_secs(60 * 60);

/// The invocation timeouts every store-backed census service declares.
///
/// Restate reads them from the manifest this endpoint publishes, per service, and falls back to its
/// own defaults — one minute without journal progress, then a ten-minute abort — when a service
/// declares neither. `BrowserSession` is deliberately not bound with these: the lane answers one
/// request at a time and a lane that hangs is better aborted on the defaults than held for an hour,
/// while a census handler that is *queued* behind the blocking pool is doing exactly what it was
/// asked to and must not be read as stalled.
fn census_service_options() -> ServiceOptions {
    ServiceOptions::new()
        .inactivity_timeout(CENSUS_INACTIVITY_TIMEOUT)
        .abort_timeout(CENSUS_ABORT_TIMEOUT)
}

/// A store-backed census service definition, with its invocation timeouts declared.
///
/// The options belong to the definition rather than to the `bind` call: that is the SDK's own
/// placement, and it keeps the timeout list next to the service it describes.
pub(super) fn census_service<D: IntoServiceDefinition>(definition: D) -> ServiceDefinition {
    definition
        .into_service_definition()
        .options(census_service_options())
}
