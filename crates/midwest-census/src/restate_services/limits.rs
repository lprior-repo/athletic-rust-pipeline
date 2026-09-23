//! The ceilings one invocation is bounded by: rows in an ingest request, windows and endpoints in a
//! sweep, rosters one jurisdiction walks in a run.
//!
//! Every one of them is admission control rather than a nicety of the code that reads it — the bound
//! is what keeps an invocation's memory and journal entry predictable, and what makes the durable
//! loops finite by construction. A caller may always ask for less; a request over a ceiling is
//! terminal, because replaying it would fail identically.

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
