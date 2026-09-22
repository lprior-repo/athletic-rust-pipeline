//! The §47 gap classes: what one jurisdiction is missing, and the count that produced the class.
//!
//! A gap row is derived, never asserted: its count is a number the jurisdiction's coverage row (or
//! the pass that filled it) measured, so a gap and the row that explains it cannot disagree. Classes
//! with a zero count are dropped, except `EmptyJurisdiction`, whose whole meaning is a jurisdiction
//! with no universe — dropping it would hide exactly the state the report exists to expose.

use super::JurisdictionCoverage;
use serde::Serialize;
use std::fmt;

/// One class of missing evidence: the unit a second pass (§47) can schedule a resolution for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GapClass {
    /// A cohort athlete with no grade observation: the cohort decision rests on the stored grad year.
    MissingGraduationEvidence,
    /// A cohort athlete with no performance row at all.
    MissingPerformanceHistory,
    /// A cohort athlete whose school holds no coach row of any sport.
    MissingCoach,
    /// A cohort athlete whose school id has no row in the schools table.
    MissingSchool,
    /// A cohort athlete with no public profile URL.
    MissingProfile,
    /// A grade observation whose implied graduation year disagrees with the stored cohort — the
    /// disagreement the athlete merge records as `Confidence::LOW`.
    ConflictingIdentity,
    /// A performance whose event id has no row in the events table.
    MissingEventContext,
    /// A cohort athlete whose every mark is unparsed (`Mark::Raw`), so no PR can be reduced.
    MissingPrSupport,
    /// A cohort athlete whose school row is absent or carries no jurisdiction.
    UnknownJurisdiction,
    /// A configured jurisdiction with no school universe and no cohort athlete.
    EmptyJurisdiction,
}

impl GapClass {
    /// The class name a sheet or a log prints, which is also the name it serializes to.
    pub const fn as_str(self) -> &'static str {
        match self {
            GapClass::MissingGraduationEvidence => "missing_graduation_evidence",
            GapClass::MissingPerformanceHistory => "missing_performance_history",
            GapClass::MissingCoach => "missing_coach",
            GapClass::MissingSchool => "missing_school",
            GapClass::MissingProfile => "missing_profile",
            GapClass::ConflictingIdentity => "conflicting_identity",
            GapClass::MissingEventContext => "missing_event_context",
            GapClass::MissingPrSupport => "missing_pr_support",
            GapClass::UnknownJurisdiction => "unknown_jurisdiction",
            GapClass::EmptyJurisdiction => "empty_jurisdiction",
        }
    }

    /// What a row's `count` counts, so a mixed list of gaps stays readable.
    pub const fn unit(self) -> &'static str {
        match self {
            GapClass::MissingEventContext => "performances",
            GapClass::EmptyJurisdiction => "schools",
            _ => "athletes",
        }
    }
}

impl fmt::Display for GapClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One class of missing evidence in one jurisdiction, with the count that produced the class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoverageGap {
    /// The jurisdiction code (`WI`), or [`UNKNOWN_JURISDICTION`](super::UNKNOWN_JURISDICTION).
    pub jurisdiction: String,
    pub class: GapClass,
    /// What `count` counts: [`GapClass::unit`].
    pub unit: &'static str,
    pub count: usize,
}

/// Gap measurements a coverage row does not carry as a column.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct GapCounters {
    /// Cohort athletes whose school id has no school row.
    pub(super) missing_school: usize,
    /// Cohort athletes whose school holds no coach row of any sport.
    pub(super) missing_coach: usize,
    /// Performances whose event id has no event row.
    pub(super) missing_event_context: usize,
}

/// Every gap row for one jurisdiction, in class declaration order.
pub(super) fn rows(row: &JurisdictionCoverage, counters: &GapCounters) -> Vec<CoverageGap> {
    let empty = row.schools == 0 && row.athletes == 0;
    let counts = [
        (GapClass::MissingGraduationEvidence, row.grad_unresolved),
        (
            GapClass::MissingPerformanceHistory,
            row.athletes.saturating_sub(row.with_performance),
        ),
        (GapClass::MissingCoach, counters.missing_coach),
        (GapClass::MissingSchool, counters.missing_school),
        (
            GapClass::MissingProfile,
            row.athletes.saturating_sub(row.with_profile_url),
        ),
        (GapClass::ConflictingIdentity, row.identity_conflicts),
        (
            GapClass::MissingEventContext,
            counters.missing_event_context,
        ),
        (
            GapClass::MissingPrSupport,
            row.with_performance
                .saturating_sub(row.with_comparable_mark),
        ),
        (GapClass::UnknownJurisdiction, unplaceable_athletes(row)),
        (GapClass::EmptyJurisdiction, 0),
    ];
    let mut gaps = Vec::new();
    for (class, count) in counts {
        if count == 0 && !(empty && class == GapClass::EmptyJurisdiction) {
            continue;
        }
        gaps.push(CoverageGap {
            jurisdiction: row.jurisdiction.clone(),
            class,
            unit: class.unit(),
            count,
        });
    }
    gaps
}

/// The athletes that landed in the unplaceable row; zero for a placed jurisdiction.
fn unplaceable_athletes(row: &JurisdictionCoverage) -> usize {
    if row.jurisdiction == super::UNKNOWN_JURISDICTION {
        row.athletes
    } else {
        0
    }
}
