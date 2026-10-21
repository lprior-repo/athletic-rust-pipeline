use super::flow;
use super::selection;
use super::source;
use census_domain::model::{CanonicalPerformance, ProfileSelectionV1};
use census_store::Store;

pub(crate) struct CoverageTimeFixture {
    pub(crate) directory: tempfile::TempDir,
    pub(crate) store: Store,
    pub(crate) selection: ProfileSelectionV1,
    pub(crate) performance: CanonicalPerformance,
}

pub(crate) fn coverage_time_fixture(
    original_centiseconds: u16,
) -> Result<CoverageTimeFixture, Box<dyn std::error::Error + Send + Sync>> {
    let original = original_centiseconds;
    let next = original.checked_add(100).ok_or("centiseconds overflow")?;
    let marks = [
        source::ControlledMark {
            time: 4900,
            date: "2026-02-28",
        },
        source::ControlledMark {
            time: original,
            date: "2026-05-15",
        },
        source::ControlledMark {
            time: next,
            date: "2026-05-16",
        },
    ];

    let installed = source::install_with_marks("Controlled Track Venue", &marks)?;

    flow::reviewed_inputs(&installed)?;

    let profile = selection::profile()?;
    installed.store.register_profile(&profile)?;

    let store_identity = installed.store.store_identity()?;
    let profile_id = profile.id()?;
    let selection = ProfileSelectionV1::new(store_identity, profile_id);

    let objects = tempfile::tempdir()?;
    let mut dataset = selection::capture(&installed.store, &profile, objects.path())?;
    let index = dataset
        .performances
        .iter()
        .position(|row| row.date == "2026-05-15")
        .ok_or("May 15 performance not found")?;
    let performance = dataset.performances.swap_remove(index);

    drop(installed.identity_writer);
    let fixture = CoverageTimeFixture {
        directory: installed.directory,
        store: installed.store,
        selection,
        performance,
    };
    Ok(fixture)
}
