use super::{JurisdictionSummary, NationalReport};
use crate::UsJurisdiction;
use std::cmp::Ordering;
use std::fmt;

pub type NationalReportResult<T, E = NationalReportError> = Result<T, E>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NationalMetric {
    Rosters,
    Athletes,
    ClassOf2027,
}

impl fmt::Display for NationalMetric {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Rosters => "rosters_total",
            Self::Athletes => "athletes_total",
            Self::ClassOf2027 => "class_of_2027_total",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NationalRows {
    Jurisdictions,
    Failures,
    Owed,
}

impl fmt::Display for NationalRows {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Jurisdictions => "jurisdictions",
            Self::Failures => "failures",
            Self::Owed => "owed",
        })
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum NationalReportError {
    #[error("national report {metric} overflows usize")]
    TotalOverflow { metric: NationalMetric },
    #[error("national report {metric} is {actual}, but jurisdiction summaries total {expected}")]
    TotalMismatch {
        metric: NationalMetric,
        expected: usize,
        actual: usize,
    },
    #[error("national report {rows} is not ordered by jurisdiction code")]
    Unordered { rows: NationalRows },
    #[error("national report {rows} repeats jurisdiction {jurisdiction}")]
    Duplicate {
        rows: NationalRows,
        jurisdiction: UsJurisdiction,
    },
    #[error("partial national report must not contain school-address completion or a final observation date")]
    PartialMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NationalTotals {
    pub rosters: usize,
    pub athletes: usize,
    pub class_of_2027: usize,
}

impl NationalTotals {
    pub fn of(rows: &[JurisdictionSummary]) -> NationalReportResult<Self> {
        let mut totals = Self {
            rosters: 0,
            athletes: 0,
            class_of_2027: 0,
        };
        for row in rows {
            totals.rosters = add(totals.rosters, row.rosters_total, NationalMetric::Rosters)?;
            totals.athletes = add(totals.athletes, row.athletes, NationalMetric::Athletes)?;
            totals.class_of_2027 = add(
                totals.class_of_2027,
                row.class_of_2027,
                NationalMetric::ClassOf2027,
            )?;
        }
        Ok(totals)
    }

    fn validate(self, report: &NationalReport) -> NationalReportResult<()> {
        for (metric, expected, actual) in [
            (NationalMetric::Rosters, self.rosters, report.rosters_total),
            (
                NationalMetric::Athletes,
                self.athletes,
                report.athletes_total,
            ),
            (
                NationalMetric::ClassOf2027,
                self.class_of_2027,
                report.class_of_2027_total,
            ),
        ] {
            if expected != actual {
                return Err(NationalReportError::TotalMismatch {
                    metric,
                    expected,
                    actual,
                });
            }
        }
        Ok(())
    }
}

impl NationalReport {
    pub fn validate(&self) -> NationalReportResult<()> {
        validate_order(&self.jurisdictions, NationalRows::Jurisdictions, |row| {
            row.jurisdiction
        })?;
        validate_order(&self.failures, NationalRows::Failures, |row| {
            row.jurisdiction
        })?;
        validate_order(&self.owed, NationalRows::Owed, |row| row.jurisdiction)?;
        if self.is_partial() && (self.school_address.is_some() || !self.today.is_empty()) {
            return Err(NationalReportError::PartialMetadata);
        }
        NationalTotals::of(&self.jurisdictions)?.validate(self)
    }
}

fn add(current: usize, increment: usize, metric: NationalMetric) -> NationalReportResult<usize> {
    current
        .checked_add(increment)
        .ok_or(NationalReportError::TotalOverflow { metric })
}

fn validate_order<T>(
    rows: &[T],
    kind: NationalRows,
    jurisdiction: impl Fn(&T) -> UsJurisdiction,
) -> NationalReportResult<()> {
    let mut previous: Option<UsJurisdiction> = None;
    for row in rows {
        let current = jurisdiction(row);
        if let Some(before) = previous {
            match before.code().cmp(current.code()) {
                Ordering::Less => {}
                Ordering::Equal => {
                    return Err(NationalReportError::Duplicate {
                        rows: kind,
                        jurisdiction: current,
                    });
                }
                Ordering::Greater => {
                    return Err(NationalReportError::Unordered { rows: kind });
                }
            }
        }
        previous = Some(current);
    }
    Ok(())
}
