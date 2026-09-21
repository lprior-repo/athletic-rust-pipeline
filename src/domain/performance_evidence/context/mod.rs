//! Source-record interpretation: event/timing/unit classification, mark parsing,
//! comparison-context assembly, and source-claim / observed-best projection.
//!
//! Every item below was previously declared in `context.rs`; the `pub(super)`
//! surface consumed by `performance_evidence` is re-exported unchanged.

mod assemble;
mod best;
mod classify;
mod mark;
mod text;

pub(super) use self::assemble::context;
pub(super) use self::best::{observed_bests, source_claims};
pub(super) use self::classify::{source_event, source_unit_for, timing_for};
pub(super) use self::mark::parse_mark;
