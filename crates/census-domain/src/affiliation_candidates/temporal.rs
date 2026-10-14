use super::{
    AffiliationCandidateError, KnownBounds, LocallyCheckedEvidence, ObjectRef, SourceSpan,
};
use crate::model::{ProfileDate, ProfileDateRangeV1, SchoolYear, Sport};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SportScope {
    AllSports,
    Sport(Sport),
}

impl SportScope {
    pub(crate) fn check(self) -> Result<(), AffiliationCandidateError> {
        if self == Self::Sport(Sport::Unknown) {
            return Err(AffiliationCandidateError::UnknownSport);
        }
        Ok(())
    }

    pub(crate) fn applies(self, sport: Sport) -> bool {
        match self {
            Self::AllSports => true,
            Self::Sport(candidate) => candidate == sport,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalGap {
    Undated,
    AmbiguousLabel,
    UnknownBoundary,
    MissingSeasonCalendar,
    OngoingContractUnresolved,
}

#[derive(Debug, Clone, Copy, Eq)]
pub struct CalendarCandidate<'a> {
    pub year: SchoolYear,
    pub sport: Sport,
    pub interval: &'a ProfileDateRangeV1,
    pub raw_label: &'a [u8],
    pub label_convention: ObjectRef<'a>,
    pub mapping_rule: &'a crate::model::ContractRefV1,
    pub evidence: &'a LocallyCheckedEvidence<'a>,
    pub locator: ObjectRef<'a>,
    pub statement: SourceSpan,
    pub known: KnownBounds<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemporalScope<'a> {
    ExactDay(&'a ProfileDate),
    Closed(&'a ProfileDateRangeV1),
    AcademicSeason {
        year: SchoolYear,
        sport: Sport,
        calendar: &'a CalendarCandidate<'a>,
    },
    ExplicitOngoingFrom {
        start: &'a ProfileDate,
        continuing_tenure: SourceSpan,
    },
    Unresolved(TemporalGap),
}

impl TemporalScope<'_> {
    pub(crate) fn check(
        self,
        scope: SportScope,
        evidence: &LocallyCheckedEvidence<'_>,
    ) -> Result<(), AffiliationCandidateError> {
        match self {
            Self::AcademicSeason {
                year,
                sport,
                calendar,
            } => {
                if sport == Sport::Unknown
                    || calendar.year != year
                    || calendar.sport != sport
                    || scope != SportScope::Sport(sport)
                {
                    return Err(AffiliationCandidateError::CalendarScopeMismatch);
                }
                if calendar.raw_label.is_empty() || calendar.raw_label.len() > 4096 {
                    return Err(AffiliationCandidateError::InvalidLocator);
                }
                calendar.statement.read(calendar.evidence)?;
            }
            Self::ExplicitOngoingFrom {
                continuing_tenure, ..
            } => {
                continuing_tenure.read(evidence)?;
            }
            Self::ExactDay(_) | Self::Closed(_) | Self::Unresolved(_) => {}
        }
        Ok(())
    }

    pub(crate) fn applies(self, as_of: &ProfileDate) -> Result<bool, TemporalGap> {
        match self {
            Self::ExactDay(day) => Ok(day == as_of),
            Self::Closed(interval) => Ok(interval.contains(as_of)),
            Self::AcademicSeason { calendar, .. } => Ok(calendar.interval.contains(as_of)),
            Self::ExplicitOngoingFrom { .. } => Err(TemporalGap::OngoingContractUnresolved),
            Self::Unresolved(gap) => Err(gap),
        }
    }

    pub(crate) fn ended_before(self, as_of: &ProfileDate) -> bool {
        match self {
            Self::Closed(interval) => interval.through() < as_of,
            Self::AcademicSeason { calendar, .. } => calendar.interval.through() < as_of,
            Self::ExactDay(_) | Self::ExplicitOngoingFrom { .. } | Self::Unresolved(_) => false,
        }
    }
}
