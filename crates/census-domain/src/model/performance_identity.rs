use super::{AthleteId, EventId, Mark, MeetId, TeamId};

pub struct PerformanceIdentity<'a> {
    pub athlete: &'a AthleteId,
    pub event: &'a EventId,
    pub meet: &'a MeetId,
    pub date: &'a str,
    pub source_key: &'a str,
}

pub struct PerformanceResult<'a> {
    pub team: &'a TeamId,
    pub mark: Mark,
    pub wind_mps: Option<f64>,
    pub place: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PerformanceError {
    #[error("performance date {value:?} is not a valid ISO date or year")]
    InvalidDate { value: String },
}

impl PerformanceIdentity<'_> {
    pub(super) fn validate(&self) -> Result<(), PerformanceError> {
        if !super::dates::valid_date(self.date) {
            return Err(PerformanceError::InvalidDate {
                value: self.date.to_owned(),
            });
        }
        Ok(())
    }

    pub(super) fn mint(&self) -> super::PerformanceId {
        super::CanonicalPerformance::mint(
            self.athlete,
            self.meet,
            self.event,
            self.date,
            self.source_key,
        )
    }
}
