use crate::keys::table_prefix;
use crate::{Entity, StoreError, StoreResult, Table, MAX_ROWS_PER_TABLE};
use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam, CollectionSnapshot, CoverageRow, RetainedConflict, ReviewCase,
    ReviewVerdictRecord, SourceAccessCondition, SourceMeetRef, SourceObjectIdentity,
    SourceObservation,
};
use fjall::{Readable, Snapshot};
use std::path::PathBuf;

mod merge;

pub struct StoreSnapshot<'s> {
    snapshot: Snapshot,
    entities: &'s fjall::Keyspace,
    root: &'s std::path::Path,
}

impl<'s> StoreSnapshot<'s> {
    pub(crate) fn new(
        snapshot: Snapshot,
        entities: &'s fjall::Keyspace,
        root: &'s std::path::Path,
    ) -> Self {
        Self {
            snapshot,
            entities,
            root,
        }
    }

    pub fn sequence(&self) -> u64 {
        self.snapshot.seqno()
    }

    pub fn for_each_observation<T: Entity>(
        &self,
        table: Table,
        mut visit: impl FnMut(T) -> StoreResult<()>,
    ) -> StoreResult<()> {
        let prefix = table_prefix(table);
        let max = usize::try_from(MAX_ROWS_PER_TABLE).map_err(|_| StoreError::CounterOverflow)?;
        self.snapshot
            .prefix(self.entities, &prefix)
            .enumerate()
            .try_for_each(|(index, guard)| {
                merge::check_limit(index, max, table)?;
                let (key, raw) = guard
                    .into_inner()
                    .map_err(|source| StoreError::Read { source })?;
                visit(merge::decode(table, &key, &raw)?)
            })
    }

    pub fn for_each_merged<T: Entity>(
        &self,
        table: Table,
        mut visit: impl FnMut(T) -> StoreResult<()>,
    ) -> StoreResult<u64> {
        let mut merged = None;
        let mut visited = 0_u64;
        self.for_each_observation(table, |row| {
            visited = merge::accept(&mut merged, row, &mut visit, visited)?;
            Ok(())
        })?;
        match merged {
            Some(row) => merge::publish(row, &mut visit, visited),
            None => Ok(visited),
        }
    }

    pub fn scan<T: Entity>(&self, table: Table) -> StoreResult<Vec<T>> {
        let mut rows = Vec::new();
        self.for_each_merged(table, |row| {
            rows.push(row);
            Ok(())
        })?;
        Ok(rows)
    }

    pub fn root(&self) -> &std::path::Path {
        self.root
    }

    pub fn out_dir(&self) -> PathBuf {
        self.root.join("out")
    }

    pub fn consolidate<T: Entity>(
        &self,
        table: Table,
        out_path: &std::path::Path,
    ) -> StoreResult<crate::Consolidated> {
        use super::snapshot::{open_snapshot_writer, publish_atomically};
        let mut rows = 0_usize;
        publish_atomically(out_path, |temporary| {
            let mut writer = open_snapshot_writer(temporary, out_path)?;
            self.for_each_merged::<T>(table, |row| {
                rows = rows.checked_add(1).ok_or(StoreError::CounterOverflow)?;
                writer.push(&row)
            })?;
            writer.finish()
        })?;
        Ok(crate::Consolidated { rows })
    }

    pub fn consolidate_table(
        &self,
        table: Table,
        out_path: &std::path::Path,
    ) -> StoreResult<crate::Consolidated> {
        match table {
            Table::Schools => self.consolidate::<CanonicalSchool>(table, out_path),
            Table::Teams => self.consolidate::<CanonicalTeam>(table, out_path),
            Table::Coaches => self.consolidate::<CanonicalCoach>(table, out_path),
            Table::Athletes => self.consolidate::<CanonicalAthlete>(table, out_path),
            Table::Meets => self.consolidate::<CanonicalMeet>(table, out_path),
            Table::Events => self.consolidate::<CanonicalEvent>(table, out_path),
            Table::Performances => self.consolidate::<CanonicalPerformance>(table, out_path),
            Table::SourceIdentities => self.consolidate::<SourceObjectIdentity>(table, out_path),
            Table::Conflicts => self.consolidate::<RetainedConflict>(table, out_path),
            Table::ReviewCases => self.consolidate::<ReviewCase>(table, out_path),
            Table::Coverage => self.consolidate::<CoverageRow>(table, out_path),
            Table::Snapshots => self.consolidate::<CollectionSnapshot>(table, out_path),
            Table::SourceAccess => self.consolidate::<SourceAccessCondition>(table, out_path),
            Table::IdentityVerdicts => self.consolidate::<ReviewVerdictRecord>(table, out_path),
            Table::SourceMeets => self.consolidate::<SourceMeetRef>(table, out_path),
            Table::SourceObservations => self.consolidate::<SourceObservation>(table, out_path),
            Table::AthleteIdentityDecisions => {
                self.consolidate::<census_domain::model::AppliedAthleteIdentity>(table, out_path)
            }
        }
    }
}

#[cfg(test)]
mod tests;
