use crate::bests::CombinedExclusionReason;
use crate::report::EventAdmission;
use census_domain::model::{
    parse_result_status, CanonicalEvent, CanonicalPerformance, CombinedComponentResult,
    CombinedEligibility, EventKind, EventSemantics, Mark, ResultStatusKind,
};

pub(super) fn admit(
    performance: &CanonicalPerformance,
    event: &CanonicalEvent,
    kind: &EventKind,
    semantics: EventSemantics,
) -> Result<(), CombinedExclusionReason> {
    if !semantics.requires_completed_combined() || !is_combined(kind) {
        return Ok(());
    }
    let specification = event
        .resolved_specification_with_policy(semantics)
        .map_err(CombinedExclusionReason::UnresolvedSpecification)?;
    match performance.combined_eligibility(&specification) {
        CombinedEligibility::Eligible => Ok(()),
        reason => Err(CombinedExclusionReason::Eligibility(reason)),
    }
}

pub(super) fn unresolved_specification(
    performance: &CanonicalPerformance,
    event: &CanonicalEvent,
    admission: &EventAdmission<'_>,
) -> Option<CombinedExclusionReason> {
    if !admission.semantics().requires_completed_combined()
        || !is_combined(&event.kind)
        || super::eligible_measure(performance, &event.kind).is_none()
    {
        return None;
    }
    event
        .resolved_specification_with_policy(admission.semantics())
        .err()
        .map(CombinedExclusionReason::UnresolvedSpecification)
}

fn is_combined(kind: &EventKind) -> bool {
    matches!(
        kind,
        EventKind::Pentathlon | EventKind::Heptathlon | EventKind::Decathlon
    )
}

pub(super) fn explicit_terminal(
    performance: &CanonicalPerformance,
    admission: &EventAdmission<'_>,
) -> bool {
    if !admission.semantics().requires_completed_combined()
        || !matches!(
            admission.included_kind(&performance.event),
            Some(EventKind::Pentathlon | EventKind::Heptathlon | EventKind::Decathlon)
        )
    {
        return false;
    }
    let Some(event) = admission.event(&performance.event) else {
        return false;
    };
    if let Mark::Raw(raw) = &performance.mark {
        if parse_result_status(raw)
            .is_some_and(|status| status.kind() == ResultStatusKind::ExplicitTerminal)
        {
            return true;
        }
    }
    terminal_completion(performance, event)
}

fn terminal_completion(performance: &CanonicalPerformance, event: &CanonicalEvent) -> bool {
    let Some(completion) = performance.combined_completion.as_ref() else {
        return false;
    };
    if event.specification.combined_event.as_ref() != Some(completion.specification()) {
        return false;
    }
    completion.components().iter().any(|component| {
        matches!(
            component,
            CombinedComponentResult::DidNotStart
                | CombinedComponentResult::DidNotFinish
                | CombinedComponentResult::Disqualified
                | CombinedComponentResult::NoMark
        )
    })
}
