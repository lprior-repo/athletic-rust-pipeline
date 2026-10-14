use super::*;

pub(super) fn reloaded_dataset(
    seed: &ExportDataset,
    observations: &[CanonicalPerformance],
) -> TestResult<ExportDataset> {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    seed.schools
        .values()
        .try_for_each(|school| store.append(Table::Schools, school))?;
    store.append_many(Table::Athletes, &seed.athletes)?;
    seed.teams
        .values()
        .try_for_each(|team| store.append(Table::Teams, team))?;
    store.append_many(Table::Meets, &seed.meets)?;
    store.append_many(Table::Events, &seed.events)?;
    store.append_many(Table::Performances, observations)?;
    let before = ExportDataset::load(&store)?;
    let selection = selected(&before)?;
    original_observations(&store, observations)?;
    store.flush()?;
    drop(store);

    let reopened = Store::open(directory.path())?;
    let after = ExportDataset::load(&reopened)?;
    check!(eq; after.performances, before.performances);
    check!(eq; selected(&after)?, selection);
    original_observations(&reopened, observations)?;
    Ok(after)
}

fn original_observations(store: &Store, expected: &[CanonicalPerformance]) -> TestResult {
    let mut physical = Vec::new();
    store.snapshot().for_each_observation(
        Table::Performances,
        |performance: CanonicalPerformance| {
            physical.push(performance);
            Ok(())
        },
    )?;
    check!(eq; physical.len(), expected.len());
    expected.iter().try_for_each(|observation| -> TestResult {
        check!(physical.contains(observation));
        Ok(())
    })
}
