//! The plan a jurisdiction census runs: one unit per source the research evidences for it, and a
//! named refusal for any of them this machine cannot acquire.
//!
//! Why a plan and not a list: the sources that apply to a state are the research corpus's conclusion
//! ([`crate::sources::applicability`]), and whether one can *run here* is a fact about this machine —
//! a browser-session source needs a lane to be configured. Those are two different dispositions and
//! the run has to carry both, so the plan is built once, before any adapter runs, and every unit
//! leaves it either sweepable or refused **by name**.
//!
//! A refusal is not a failure. It is terminal for the run, it is owed rather than produced, and it
//! must not reach the invocation retry: a source that cannot run is not a source that failed, and
//! spending three attempts on a missing lane would report a machine's gap as a source's fault.

use crate::net::Fetcher;
use crate::sources::applicability::applicable_sources;
use crate::sources::registry::{AccessClass, SourceDescriptor};
use census_domain::UsJurisdiction;

/// Whether this machine can serve a browser-session source at plan time.
///
/// Plan-time only, on purpose: a lane that exists but has no browser behind it is the run-time case,
/// which the transport records as an access condition against the host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrowserLaneState {
    /// A lane is configured: a browser-session source is planned like any other.
    Configured,
    /// No lane: a browser-session source is refused, naming what is missing.
    Absent,
}

impl BrowserLaneState {
    /// This machine's lane state, as the transport the run will fetch through reports it.
    ///
    /// The run asks its own fetcher instead of a setting read somewhere else, so the plan refuses a
    /// browser-transported source exactly when the fetcher would have no lane to hand it to.
    pub fn of(fetcher: &Fetcher) -> Self {
        if fetcher.has_browser_lane() {
            Self::Configured
        } else {
            Self::Absent
        }
    }
}

/// One unit the run can sweep: the adapter's slug, and how its bytes are acquired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedUnit {
    /// The adapter's slug, exactly as the registry resolves it. The unit's identity in a report.
    pub slug: &'static str,
    /// How this source is acquired, recorded durably beside the unit.
    pub access: AccessClass,
}

/// What the plan owes for one applicable source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitDisposition {
    /// The source can be acquired on this machine.
    Sweep(PlannedUnit),
    /// Applicable, but this machine cannot run it. Terminal for the run, owed for a later one, and
    /// never routed through the invocation retry: the reason names what is missing.
    Refused(Refusal),
}

/// The reason a browser-session source is refused when no lane is configured. One string, so the
/// report, the log line, and the test all quote the same sentence.
const NO_BROWSER_LANE: &str =
    "source needs a browser session and no browser lane is configured: start a lane and re-run";

impl UnitDisposition {
    /// The slug of the source this disposition is about, sweepable or refused.
    pub fn slug(&self) -> &'static str {
        match self {
            Self::Sweep(unit) => unit.slug,
            Self::Refused(refusal) => refusal.slug,
        }
    }
}

/// Plan one jurisdiction's units: every source the applicability table evidences for it, in the
/// table's planning order, each classified against this machine.
///
/// A jurisdiction the research does not evidence plans nothing — an empty plan is a real answer and
/// must not be read as "plan everything"; [`applicable_sources`] is total over the scope and already
/// answers empty for one.
pub fn plan(jurisdiction: UsJurisdiction, lane: BrowserLaneState) -> Vec<UnitDisposition> {
    plan_sources(&applicable_sources(jurisdiction), lane)
}

/// Plan the units for descriptors a caller already holds. [`plan`] reaches this through
/// [`applicable_sources`], so the refusal path a run takes is the one a test drives.
pub fn plan_sources(
    descriptors: &[&'static SourceDescriptor],
    lane: BrowserLaneState,
) -> Vec<UnitDisposition> {
    descriptors
        .iter()
        .map(|descriptor| classify(descriptor, lane))
        .collect()
}

/// A refusal, named so a run can carry it into a record: which source, how it would have been
/// acquired, and what this machine is missing.
///
/// A refusal is never routed through the invocation retry declared at `restate_services/census.rs:46`:
/// that retry exists for inputs that may still be there and `JobError::Transient`
/// (`restate_services/support.rs:20`) is its only feed. A missing browser lane is neither - it is
/// owed work for a later run: recorded as evidence, not retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Refusal {
    /// The adapter's slug.
    pub slug: &'static str,
    /// How the source would have been acquired, so the refusal reads as a machine gap.
    pub access: AccessClass,
    /// What is missing, in the operator's vocabulary.
    pub reason: &'static str,
}

/// The units a dispatcher may fetch: everything the plan did not refuse.
pub fn sweepable(dispositions: &[UnitDisposition]) -> Vec<PlannedUnit> {
    dispositions
        .iter()
        .filter_map(|disposition| match disposition {
            UnitDisposition::Sweep(unit) => Some(*unit),
            UnitDisposition::Refused(_) => None,
        })
        .collect()
}

/// The refusals, in plan order: the owed work a run records instead of dispatching.
pub fn owed(dispositions: &[UnitDisposition]) -> Vec<Refusal> {
    dispositions
        .iter()
        .filter_map(|disposition| match disposition {
            UnitDisposition::Sweep(_) => None,
            UnitDisposition::Refused(refusal) => Some(*refusal),
        })
        .collect()
}

/// Classify one applicable descriptor against this machine's capability.
fn classify(descriptor: &'static SourceDescriptor, lane: BrowserLaneState) -> UnitDisposition {
    classify_access(descriptor.slug, descriptor.access_class(), lane)
}

/// The decision, split from the descriptor so a test can drive the browser-session branch with a
/// literal class: no registry entry declares a browser transport yet, so the branch is otherwise
/// unreachable from the real table.
pub(super) fn classify_access(
    slug: &'static str,
    access: AccessClass,
    lane: BrowserLaneState,
) -> UnitDisposition {
    if access == AccessClass::BrowserSession && lane == BrowserLaneState::Absent {
        return UnitDisposition::Refused(Refusal {
            slug,
            access,
            reason: NO_BROWSER_LANE,
        });
    }
    UnitDisposition::Sweep(PlannedUnit { slug, access })
}

#[cfg(test)]
mod tests;
