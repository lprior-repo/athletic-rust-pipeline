use super::super::routing::TargetIndex;
use super::super::{MeetRoute, PlanAllocation, PlannedOwnedTarget, ResultsPlanError};
use census_domain::model::SourceMeetRef;

pub(in crate::restate_services) fn meets(
    input: Vec<SourceMeetRef>,
    budget: &mut PlanAllocation,
) -> Result<Vec<SourceMeetRef>, ResultsPlanError> {
    let mut keyed = Vec::new();
    budget.reserve(&mut keyed, input.len())?;
    for meet in input {
        let count = super::super::codec::writer::count(&meet)?;
        budget.charge(count)?;
        let key = super::super::codec::writer::encode(&meet, count)?;
        keyed.push((key, meet));
    }
    keyed.sort_unstable_by(|left, right| left.0.cmp(&right.0));
    let mut output = Vec::new();
    budget.reserve(&mut output, keyed.len())?;
    for (_, meet) in keyed {
        output.push(meet);
    }
    Ok(output)
}

pub(in crate::restate_services) fn targets(
    targets: &mut Vec<PlannedOwnedTarget>,
    routes: &mut [MeetRoute],
    budget: &mut PlanAllocation,
) -> Result<(), ResultsPlanError> {
    let mut order = Vec::new();
    budget.reserve(&mut order, targets.len())?;
    for (position, target) in targets.drain(..).enumerate() {
        order.push((position, target));
    }
    order.sort_unstable_by(|left, right| left.1.slot_id.cmp(&right.1.slot_id));
    let mut positions = Vec::new();
    budget.reserve(&mut positions, order.len())?;
    positions.resize(order.len(), TargetIndex::from_position(0)?);
    for (new, (old, target)) in order.into_iter().enumerate() {
        *positions
            .get_mut(old)
            .ok_or(ResultsPlanError::Invalid("target permutation"))? =
            TargetIndex::from_position(new)?;
        targets.push(target);
    }
    for route in routes {
        if let MeetRoute::Owned(index) = route {
            *index = *positions
                .get(index.position()?)
                .ok_or(ResultsPlanError::Invalid("target permutation"))?;
        }
    }
    Ok(())
}
