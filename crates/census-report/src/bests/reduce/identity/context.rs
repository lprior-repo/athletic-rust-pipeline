use census_domain::model::{
    AdjustmentBasis, AdjustmentKind, AthleteId, CanonicalPerformance, EventId, Grade, MeetId,
    PerformanceId, SourceNamespace, TeamId, TimingMethod,
};
use serde::Serialize;

#[derive(PartialEq, Eq, Serialize)]
pub(super) struct IdentityContext<'a> {
    id: &'a PerformanceId,
    athlete: &'a AthleteId,
    team: &'a TeamId,
    event: &'a EventId,
    meet: &'a MeetId,
    date: &'a str,
    source_key: &'a str,
    heat: Option<&'a str>,
    round: Option<&'a str>,
    timing: Option<TimingMethod>,
    wind_bits: Option<u64>,
    place: Option<u16>,
    observed_grade: Option<Grade>,
    owner: Option<(&'a SourceNamespace, &'a str)>,
    adjustment: Option<(AdjustmentKind, AdjustmentBasis)>,
}

impl<'a> IdentityContext<'a> {
    pub(super) fn of(row: &'a CanonicalPerformance) -> Self {
        Self {
            id: &row.id,
            athlete: &row.athlete,
            team: &row.team,
            event: &row.event,
            meet: &row.meet,
            date: &row.date,
            source_key: &row.source_key,
            heat: row.heat.as_deref(),
            round: row.round.as_deref(),
            timing: row.timing,
            wind_bits: row.wind_mps.map(f64::to_bits),
            place: row.place,
            observed_grade: row.observed_grade,
            owner: row
                .source_athlete
                .as_ref()
                .map(|source| (&source.namespace, source.id.as_str())),
            adjustment: row
                .adjustment
                .as_ref()
                .map(|value| (value.kind, value.basis)),
        }
    }
}
