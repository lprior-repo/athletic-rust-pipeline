use super::JurisdictionCoverage;
use census_domain::JurisdictionBucket;
use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GapClass {
    MissingGraduationEvidence,
    MissingPerformanceHistory,
    MissingCoach,
    MissingSchool,
    MissingProfile,
    ConflictingIdentity,
    MissingEventContext,
    UnmappedEvent,
    MissingPrSupport,
    UnknownJurisdiction,
    EmptyJurisdiction,
}

impl GapClass {
    pub const fn as_str(self) -> &'static str {
        match self {
            GapClass::MissingGraduationEvidence => "missing_graduation_evidence",
            GapClass::MissingPerformanceHistory => "missing_performance_history",
            GapClass::MissingCoach => "missing_coach",
            GapClass::MissingSchool => "missing_school",
            GapClass::MissingProfile => "missing_profile",
            GapClass::ConflictingIdentity => "conflicting_identity",
            GapClass::MissingEventContext => "missing_event_context",
            GapClass::UnmappedEvent => "unmapped_event",
            GapClass::MissingPrSupport => "missing_pr_support",
            GapClass::UnknownJurisdiction => "unknown_jurisdiction",
            GapClass::EmptyJurisdiction => "empty_jurisdiction",
        }
    }

    pub const fn unit(self) -> &'static str {
        match self {
            GapClass::MissingEventContext => "performances",
            GapClass::UnmappedEvent => "performances",
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoverageGap {
    pub jurisdiction: JurisdictionBucket,
    pub class: GapClass,
    pub unit: &'static str,
    pub count: usize,
}

#[derive(Debug, Default, Clone, Copy)]
pub(super) struct GapCounters {
    pub(super) missing_school: usize,
    pub(super) missing_coach: usize,
    pub(super) missing_event_context: usize,
    pub(super) unmapped_event: usize,
}

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
        (GapClass::UnmappedEvent, counters.unmapped_event),
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
            jurisdiction: row.jurisdiction,
            class,
            unit: class.unit(),
            count,
        });
    }
    gaps
}

fn unplaceable_athletes(row: &JurisdictionCoverage) -> usize {
    if row.jurisdiction == JurisdictionBucket::Unplaced {
        row.athletes
    } else {
        0
    }
}
