use super::*;
use crate::bests::SharedSelection;
use census_domain::model::performance_comparison::{
    Measure, PrKey, SurfaceClass, TimingClass, WindClass,
};
use std::collections::BTreeMap;

mod context_refusal;
mod long_flat_selection;
mod policy_compatibility;

type LongFlatWinners = BTreeMap<PrKey, (PerformanceId, MeetId, i64, usize)>;

fn long_flat_key(
    athlete: &AthleteId,
    kind: EventKind,
    surface: SurfaceClass,
    timing: TimingClass,
    indoor_track: Option<IndoorTrackSpecification>,
) -> PrKey {
    PrKey {
        athlete_id: athlete.clone(),
        event_kind: kind,
        surface,
        wind_class: WindClass::NotApplicable,
        timing,
        measure: Measure::Time,
        context: None,
        specification: EventSpecification {
            category: Some(CompetitionCategory::Boys),
            indoor_track,
            ..EventSpecification::default()
        },
        comparison: ComparisonPolicy::Standard,
        adjustment: None,
    }
}

fn check_long_flat_winners(rows: &[SharedSelection], expected: &LongFlatWinners) -> TestResult {
    check!(eq; rows.len(), expected.len());
    let actual: LongFlatWinners = rows
        .iter()
        .map(|row| {
            (
                row.key.clone(),
                (
                    row.source.performance_id.clone(),
                    row.meet.meet_id.clone(),
                    row.result.value,
                    row.population.marks,
                ),
            )
        })
        .collect();
    check!(eq; &actual, expected);
    Ok(())
}

fn check_long_flat_refusal(
    data: &crate::export::ExportDataset,
    reason: EventResolutionReason,
) -> TestResult {
    let event = data.events.first().ok_or("refused event")?;
    let performance = data.performances.first().ok_or("refused performance")?;
    let meet = data.meets.first().ok_or("refused meet")?;
    let sport = *meet.sports.first().ok_or("refused sport")?;
    check!(eq; RecruitingCatalogV1::new().classify(event, sport)?, EventDisposition::Unresolved(reason));
    check!(eq; PrKey::from_performance(performance, event, Some(meet), Measure::Time, EventSemantics::FlatEvents19)?, None);
    check_long_flat_winners(&selections(data)?, &LongFlatWinners::new())
}
