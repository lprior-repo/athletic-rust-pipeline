use census_domain::model::{
    normalize_name, Mark, RelayMember, RelayResult, RetainedConflict, SourceObservation,
    RELAY_MEMBER_NAME_FAMILY,
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
        if matches!(self.mark, Mark::Raw(_)) && !matches!(other.mark, Mark::Raw(_)) {
            self.mark = other.mark;
        }
        if self.place.is_none() {
            self.place = other.place;
        }
        if self.round.is_none() {
            self.round = other.round;
        }
        if self.timing.is_none() {
            self.timing = other.timing;
        }
        if self.source_team.is_none() {
            self.source_team = other.source_team;
        }
        other
            .retained_conflicts
            .into_iter()
            .for_each(|conflict| record(&mut self.retained_conflicts, conflict));
        other.members.into_iter().for_each(|member| {
            let order = member.order;
            match self.members.iter_mut().find(|held| held.order == order) {
                None => self.members.push(member),
                Some(held) => {
                    if same_spelling(&held.name_as_published, &member.name_as_published) {
                        if held.athlete.is_none() {
                            held.athlete = member.athlete;
                        } else if let (Some(kept), Some(incoming)) =
                            (&held.athlete, &member.athlete)
                        {
                            if kept != incoming {
                                let conflict =
                                    member_conflict(self.id.as_str(), order, held, &member);
                                record(&mut self.retained_conflicts, conflict);
                            }
                        }
                    } else {
                        let conflict = member_conflict(self.id.as_str(), order, held, &member);
                        record(&mut self.retained_conflicts, conflict);
                    }
                }
            }
        });
        self.members.sort_by_key(|member| member.order);
        super::union_vec(&mut self.evidence, other.evidence);
    }
}

fn record(conflicts: &mut Vec<RetainedConflict>, conflict: RetainedConflict) {
    if !conflicts.contains(&conflict) {
        conflicts.push(conflict);
    }
}

fn same_spelling(left: &str, right: &str) -> bool {
    normalize_name(left) == normalize_name(right)
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
