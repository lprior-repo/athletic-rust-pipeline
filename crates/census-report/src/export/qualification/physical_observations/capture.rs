use super::*;

pub(super) fn capture<T: Entity>(
    dataset: &ExportDataset,
    snapshot: &StoreSnapshot<'_>,
    table: Table,
    objects_dir: &Path,
    budget: ReadBudget,
) -> ReportResult<PhysicalObservationDescriptor> {
    let mut writer = Writer::new(dataset, snapshot, table, objects_dir, budget)?;
    let mut failure = None;
    let scanned =
        snapshot.for_each_physical_observation_bounded(table, budget, |_: T, key, raw, _| {
            if let Err(error) = writer.push(key, raw) {
                failure = Some(error);
                return Err(invalid("physical observation writer refused"));
            }
            Ok(())
        });
    if let Some(error) = failure {
        return Err(error);
    }
    writer.finish(scanned?)
}
