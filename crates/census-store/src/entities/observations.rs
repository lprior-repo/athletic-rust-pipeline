use census_domain::model::{
    normalize_name, Mark, RelayMember, RelayResult, RetainedConflict, SourceObservation,
    RELAY_MEMBER_NAME_FAMILY, RELAY_RESULT_SCALAR_FAMILY,
};

use super::super::Entity;

impl Entity for SourceObservation {
    fn entity_id(&self) -> &str {
        self.id()
    }

    fn merge(&mut self, other: Self) {
        self.absorb(other);
    }
}

impl Entity for RelayResult {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        merge_relay_mark(self, &other);
        self.place = merge_scalar(
            self.id.as_str(),
            &mut self.retained_conflicts,
            "place",
            self.place,
            other.place,
        );
        self.round = merge_scalar(
            self.id.as_str(),
            &mut self.retained_conflicts,
            "round",
            self.round.clone(),
            other.round,
        );
        self.timing = merge_scalar(
            self.id.as_str(),
            &mut self.retained_conflicts,
            "timing",
            self.timing,
            other.timing,
        );
        if self.source_team.is_none() {
            self.source_team = other.source_team;
        }
        other
            .retained_conflicts
            .into_iter()
            .for_each(|conflict| record(&mut self.retained_conflicts, conflict));
        absorb_relay_members(self, other.members);
        self.members.sort_by_key(|member| member.order);
        super::union_vec(&mut self.evidence, other.evidence);
    }
}

fn merge_relay_mark(entity: &mut RelayResult, other: &RelayResult) {
    if matches!(entity.mark, Mark::Raw(_)) && !matches!(other.mark, Mark::Raw(_)) {
        entity.mark = other.mark.clone();
    }
    if entity.mark != other.mark && !matches!(other.mark, Mark::Raw(_)) {
        let conflict = scalar_conflict(entity.id.as_str(), "mark", &entity.mark, &other.mark);
        record(&mut entity.retained_conflicts, conflict);
    }
}

fn absorb_relay_members(entity: &mut RelayResult, members: Vec<RelayMember>) {
    for member in members {
        let order = member.order;
        match entity.members.iter_mut().find(|held| held.order == order) {
            None => entity.members.push(member),
            Some(held) => {
                if same_spelling(&held.name_as_published, &member.name_as_published) {
                    if held.athlete.is_none() {
                        held.athlete = member.athlete;
                    } else if let (Some(kept), Some(incoming)) = (&held.athlete, &member.athlete) {
                        if kept != incoming {
                            let conflict =
                                member_conflict(entity.id.as_str(), order, held, &member);
                            record(&mut entity.retained_conflicts, conflict);
                        }
                    }
                } else {
                    let conflict = member_conflict(entity.id.as_str(), order, held, &member);
                    record(&mut entity.retained_conflicts, conflict);
                }
            }
        }
    }
}

fn merge_scalar<T: Clone + PartialEq + std::fmt::Debug>(
    entity_id: &str,
    conflicts: &mut Vec<RetainedConflict>,
    field: &'static str,
    kept: Option<T>,
    dropped: Option<T>,
) -> Option<T> {
    if let (Some(kept_value), Some(dropped_value)) = (&kept, &dropped) {
        if kept_value != dropped_value {
            let conflict = scalar_conflict(entity_id, field, &kept, &dropped);
            record(conflicts, conflict);
        }
    }
    kept.or(dropped)
}

fn record(conflicts: &mut Vec<RetainedConflict>, conflict: RetainedConflict) {
    if !conflicts.contains(&conflict) {
        conflicts.push(conflict);
    }
}

fn same_spelling(left: &str, right: &str) -> bool {
    normalize_name(left) == normalize_name(right)
}

fn scalar_conflict(
    relay_id: &str,
    field: &'static str,
    kept: &impl std::fmt::Debug,
    dropped: &impl std::fmt::Debug,
) -> RetainedConflict {
    RetainedConflict::new(
        RELAY_RESULT_SCALAR_FAMILY,
        relay_id,
        field,
        format!("{field} kept {kept:?} dropped {dropped:?}"),
    )
}

fn member_conflict(
    relay_id: &str,
    order: u32,
    kept: &RelayMember,
    incoming: &RelayMember,
) -> RetainedConflict {
    RetainedConflict::new(
        RELAY_MEMBER_NAME_FAMILY,
        relay_id,
        format!("order {order}"),
        format!(
            "order {order} kept {:?} ({:?}) dropped {:?} ({:?})",
            kept.name_as_published, kept.athlete, incoming.name_as_published, incoming.athlete
        ),
    )
}

#[cfg(test)]
#[path = "observations_tests.rs"]
mod tests;
