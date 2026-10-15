use census_domain::model::{
    Adjustment, AdjustmentBasis, AdjustmentKind, CanonicalPerformance, CentiMetres, CentiPoints,
    Evidence, EvidenceMethod, Mark, SourceRef,
};
use std::cmp::Ordering;

pub(super) fn compare(left: &CanonicalPerformance, right: &CanonicalPerformance) -> Ordering {
    left.evidence
        .iter()
        .map(EvidenceOrder::of)
        .cmp(right.evidence.iter().map(EvidenceOrder::of))
        .then_with(|| {
            left.adjustment
                .as_ref()
                .map(AdjustmentOrder::of)
                .cmp(&right.adjustment.as_ref().map(AdjustmentOrder::of))
        })
        .then_with(|| ExactMarkOrder::of(&left.mark).cmp(&ExactMarkOrder::of(&right.mark)))
        .then_with(|| left.source_athlete.cmp(&right.source_athlete))
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct EvidenceOrder<'a> {
    source: &'a SourceRef,
    observed_on: &'a str,
    method: &'static str,
    note: Option<&'a str>,
}

impl<'a> EvidenceOrder<'a> {
    fn of(evidence: &'a Evidence) -> Self {
        Self {
            source: &evidence.source,
            observed_on: &evidence.observed_on,
            method: match evidence.method {
                EvidenceMethod::Derived => "derived",
                EvidenceMethod::Fetched => "fetched",
                EvidenceMethod::Parsed => "parsed",
            },
            note: evidence.note.as_deref(),
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct AdjustmentOrder<'a> {
    kind: AdjustmentKind,
    basis: AdjustmentBasis,
    original: Option<ExactMarkOrder<'a>>,
    adjusted: ExactMarkOrder<'a>,
    note: Option<&'a str>,
    source: Option<&'a SourceRef>,
}

impl<'a> AdjustmentOrder<'a> {
    fn of(adjustment: &'a Adjustment) -> Self {
        Self {
            kind: adjustment.kind,
            basis: adjustment.basis,
            original: adjustment.original_mark.as_ref().map(ExactMarkOrder::of),
            adjusted: ExactMarkOrder::of(&adjustment.adjusted_mark),
            note: adjustment.note.as_deref(),
            source: adjustment.source.as_ref(),
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum ExactMarkOrder<'a> {
    Distance(&'a CentiMetres),
    Imperial(&'a str, &'a CentiMetres),
    Points(&'a CentiPoints),
    Raw(&'a str),
    Time(i64, u8),
}

impl<'a> ExactMarkOrder<'a> {
    fn of(mark: &'a Mark) -> Self {
        match mark {
            Mark::DistanceMetres(value) => Self::Distance(value),
            Mark::FieldImperial { feet_mark, metres } => Self::Imperial(feet_mark, metres),
            Mark::Points(value) => Self::Points(value),
            Mark::Raw(value) => Self::Raw(value),
            Mark::TimeSeconds(value) => Self::Time(value.value(), value.precision()),
        }
    }
}
