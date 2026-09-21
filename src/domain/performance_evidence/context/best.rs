//! Projection of source best claims and of the best marks observed inside the
//! supplied sample, grouped by comparable context.

use super::super::{
    AttributionContext, BestClaim, BestClaimKind, EquipmentContext, MarkObservation,
    ObservedBestGroup, PerformanceContext, PerformanceObservation, ResultEvidence, SourceBestClaim,
    SourceUnit, SurfaceContext, TimingBasis, WindLegality,
};
use crate::domain::evidence::Sport;
use crate::domain::marks::{compare_performances, Comparison, EventName};

pub(in super::super) fn source_claims(source: &ResultEvidence) -> Vec<SourceBestClaim> {
    [
        (BestClaimKind::PersonalBest, source.personal_best.clone()),
        (BestClaimKind::SeasonBest, source.season_best.clone()),
    ]
    .into_iter()
    .map(|(kind, raw)| SourceBestClaim {
        result_id: source.result_id,
        kind: kind.clone(),
        claimed: claim_value(&raw),
        raw,
        evidence: source.evidence.clone(),
    })
    .collect()
}

fn claim_value(claim: &BestClaim) -> Option<bool> {
    match claim {
        BestClaim::Claimed => Some(true),
        BestClaim::NotClaimed => Some(false),
        // Athletic.net's numeric TF flags have no grounded public semantics in
        // this pipeline. Preserve the raw value, but do not infer a boolean.
        BestClaim::OpaqueFlags(_) | BestClaim::Unavailable => None,
    }
}

pub(in super::super) fn observed_bests(
    observations: &[PerformanceObservation],
) -> Vec<ObservedBestGroup> {
    observations
        .iter()
        .filter(|item| eligible(item))
        .fold(Vec::new(), |mut groups, observation| {
            let group = groups
                .iter_mut()
                .find(|item| comparable_context(&item.context, &observation.context));
            match group {
                Some(group) if improves(observation, &group.best) => {
                    group.best = observation.clone()
                }
                Some(_) => {}
                None => groups.push(ObservedBestGroup {
                    context: observation.context.clone(),
                    best: observation.clone(),
                }),
            }
            groups
        })
}

fn eligible(observation: &PerformanceObservation) -> bool {
    observation.result_id > 0
        && matches!(
            &observation.context.attribution,
            AttributionContext::Individual
        )
        && known_context(&observation.context)
        && matches!(&observation.mark, MarkObservation::Parsed(mark) if mark.comparable().is_some())
}

fn comparable_context(left: &PerformanceContext, right: &PerformanceContext) -> bool {
    left == right && known_context(left)
}

fn known_context(context: &PerformanceContext) -> bool {
    let timing_known = matches!(
        &context.timing,
        TimingBasis::Fat | TimingBasis::Hand | TimingBasis::NotApplicable
    );
    let wind_known = matches!(
        &context.wind,
        WindLegality::Legal | WindLegality::Illegal | WindLegality::NotApplicable
    );
    let event_type_known = context.sport == Sport::CrossCountry || context.event_type.is_some();
    let equipment_known = if context.event.as_str().ends_with('h') {
        matches!(&context.equipment, EquipmentContext::Hurdles(_))
    } else if matches!(
        context.event,
        EventName::ShotPut
            | EventName::Discus
            | EventName::Javelin
            | EventName::Hammer
            | EventName::WeightThrow
    ) {
        matches!(&context.equipment, EquipmentContext::Implement(_))
    } else {
        true
    };
    context.distance.is_some()
        && !matches!(&context.surface, SurfaceContext::Unknown)
        && timing_known
        && !matches!(&context.units, SourceUnit::Unknown)
        && wind_known
        && event_type_known
        && equipment_known
}

fn improves(candidate: &PerformanceObservation, previous: &PerformanceObservation) -> bool {
    match (&candidate.mark, &previous.mark) {
        (MarkObservation::Parsed(left), MarkObservation::Parsed(right)) => {
            compare_performances(left, right) == Comparison::Better
        }
        _ => false,
    }
}
