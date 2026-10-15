use super::reports::CompetitionContext;
use census_domain::model::{CanonicalPerformance, EventSemantics, PerformanceId};

#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum ReportGroupKey<'a> {
    HistoricalContext(CompetitionContext<'a>),
    CanonicalPerformance(&'a PerformanceId),
}

pub(super) fn terminal(
    performance: &CanonicalPerformance,
    admission: &crate::report::EventAdmission<'_>,
) -> bool {
    if super::combined::explicit_terminal(performance, admission) {
        return true;
    }
    if admission.semantics() != EventSemantics::ResultGroups21
        || admission.included_kind(&performance.event).is_none()
    {
        return false;
    }
    matches!(&performance.mark, census_domain::model::Mark::Raw(raw)
        if census_domain::model::parse_result_status(raw).is_some_and(|status|
            status.kind() == census_domain::model::ResultStatusKind::ExplicitTerminal))
}

impl<'a> ReportGroupKey<'a> {
    pub(super) fn of(performance: &'a CanonicalPerformance, policy: EventSemantics) -> Self {
        match policy {
            EventSemantics::Legacy18
            | EventSemantics::FlatEvents19
            | EventSemantics::CombinedEvents20 => {
                Self::HistoricalContext(CompetitionContext::of(performance))
            }
            EventSemantics::ResultGroups21 => Self::CanonicalPerformance(&performance.id),
        }
    }
}

#[cfg(test)]
mod tests;
