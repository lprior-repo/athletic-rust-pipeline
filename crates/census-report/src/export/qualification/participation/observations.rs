use super::{closure::Closure, invalid};
use crate::export::qualification::physical_observations::{self, PhysicalObservationDescriptor};
use crate::export::ExportDataset;
use crate::report::ReportResult;
use census_domain::model::{
    CanonicalAthlete, CanonicalEvent, CanonicalMeet, CanonicalPerformance, CanonicalTeam,
};
use census_store::{Entity, ReadBudget, StoreSnapshot, Table};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) const BUDGET: ReadBudget = ReadBudget {
    rows: 200_000,
    bytes: 256 * 1024 * 1024,
    record_bytes: 1024 * 1024,
};
const TABLES: [Table; 5] = [
    Table::Athletes,
    Table::Performances,
    Table::Teams,
    Table::Meets,
    Table::Events,
];

pub(super) fn capture(
    dataset: &ExportDataset,
    snapshot: &StoreSnapshot<'_>,
    directory: &Path,
    athletes: &PhysicalObservationDescriptor,
    expected_athletes: &str,
) -> ReportResult<(Closure, Vec<PhysicalObservationDescriptor>)> {
    if athletes.table() != Table::Athletes {
        return Err(invalid("participation requires the retained athlete stream").into());
    }
    let descriptors = vec![
        athletes.clone(),
        physical_observations::capture::<CanonicalPerformance>(
            dataset,
            snapshot,
            Table::Performances,
            directory,
            BUDGET,
        )?,
        physical_observations::capture::<CanonicalTeam>(
            dataset,
            snapshot,
            Table::Teams,
            directory,
            BUDGET,
        )?,
        physical_observations::capture::<CanonicalMeet>(
            dataset,
            snapshot,
            Table::Meets,
            directory,
            BUDGET,
        )?,
        physical_observations::capture::<CanonicalEvent>(
            dataset,
            snapshot,
            Table::Events,
            directory,
            BUDGET,
        )?,
    ];
    let mut expected = fingerprints(&descriptors);
    expected.insert(Table::Athletes.file().into(), expected_athletes.into());
    let closure = read(dataset, directory, &descriptors, &expected)?;
    Ok((closure, descriptors))
}

pub(super) fn fingerprints(
    descriptors: &[PhysicalObservationDescriptor],
) -> BTreeMap<String, String> {
    descriptors
        .iter()
        .map(|descriptor| {
            (
                descriptor.table().file().into(),
                descriptor.fingerprint().into(),
            )
        })
        .collect()
}

pub(super) fn read(
    dataset: &ExportDataset,
    directory: &Path,
    descriptors: &[PhysicalObservationDescriptor],
    expected: &BTreeMap<String, String>,
) -> ReportResult<Closure> {
    if descriptors.len() != TABLES.len()
        || TABLES.iter().any(|table| {
            descriptors
                .iter()
                .filter(|descriptor| descriptor.table() == *table)
                .count()
                != 1
                || !expected.contains_key(table.file())
        })
    {
        return Err(
            invalid("participation physical closure omits or repeats a required table").into(),
        );
    }
    Ok(Closure {
        athletes: rows::<CanonicalAthlete>(
            dataset,
            directory,
            descriptors,
            expected,
            Table::Athletes,
        )?,
        performances: rows::<CanonicalPerformance>(
            dataset,
            directory,
            descriptors,
            expected,
            Table::Performances,
        )?,
        teams: rows::<CanonicalTeam>(dataset, directory, descriptors, expected, Table::Teams)?,
        meets: rows::<CanonicalMeet>(dataset, directory, descriptors, expected, Table::Meets)?,
        events: rows::<CanonicalEvent>(dataset, directory, descriptors, expected, Table::Events)?,
    })
}

fn rows<T: Entity>(
    dataset: &ExportDataset,
    directory: &Path,
    descriptors: &[PhysicalObservationDescriptor],
    expected: &BTreeMap<String, String>,
    table: Table,
) -> ReportResult<Vec<T>> {
    let descriptor = descriptors
        .iter()
        .find(|descriptor| descriptor.table() == table)
        .ok_or_else(|| invalid("physical descriptor missing"))?;
    let fingerprint = expected
        .get(table.file())
        .ok_or_else(|| invalid("root-bound physical fingerprint missing"))?;
    let mut rows = Vec::new();
    physical_observations::visit(dataset, directory, descriptor, fingerprint, BUDGET, |row| {
        rows.try_reserve(1)
            .map_err(|_| invalid("participation physical observation allocation refused"))?;
        rows.push(row);
        Ok(())
    })?;
    Ok(rows)
}
