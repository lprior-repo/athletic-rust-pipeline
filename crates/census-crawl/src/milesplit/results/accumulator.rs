use super::Accumulator;
use census_store::{Entity, Table};
use std::collections::{HashMap, HashSet};

impl Accumulator {
    pub(super) fn absorb(&mut self, other: Self) {
        absorb(
            &mut self.meets,
            other.meets,
            Table::Meets,
            &mut self.pending,
        );
        absorb(
            &mut self.events,
            other.events,
            Table::Events,
            &mut self.pending,
        );
        absorb(
            &mut self.teams,
            other.teams,
            Table::Teams,
            &mut self.pending,
        );
        absorb(
            &mut self.athletes,
            other.athletes,
            Table::Athletes,
            &mut self.pending,
        );
        absorb(
            &mut self.performances,
            other.performances,
            Table::Performances,
            &mut self.pending,
        );
        absorb(
            &mut self.observations,
            other.observations,
            Table::SourceObservations,
            &mut self.pending,
        );
        other.retained.into_iter().for_each(|(key, value)| {
            self.pending
                .entry("retained")
                .or_default()
                .insert(key.clone());
            self.retained.insert(key, value);
        });
    }
    pub(super) fn rows(&self) -> usize {
        [
            self.meets.len(),
            self.events.len(),
            self.teams.len(),
            self.athletes.len(),
            self.performances.len(),
            self.observations.len(),
            self.retained.len(),
        ]
        .into_iter()
        .fold(0usize, usize::saturating_add)
    }
}

fn absorb<T: Entity>(
    held: &mut HashMap<String, T>,
    incoming: HashMap<String, T>,
    table: Table,
    pending: &mut HashMap<&'static str, HashSet<String>>,
) {
    incoming.into_iter().for_each(|(key, value)| {
        pending.entry(table.file()).or_default().insert(key.clone());
        match held.entry(key) {
            std::collections::hash_map::Entry::Occupied(mut entry) => entry.get_mut().merge(value),
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(value);
            }
        }
    });
}

pub(super) fn pending_rows<T>(
    rows: HashMap<String, T>,
    table: Table,
    pending: &HashMap<&'static str, HashSet<String>>,
) -> Vec<T> {
    rows.into_iter()
        .filter_map(|(key, value)| {
            pending
                .get(table.file())
                .is_some_and(|keys| keys.contains(&key))
                .then_some(value)
        })
        .collect()
}
