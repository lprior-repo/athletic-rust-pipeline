use super::{ExportDataset, GradYear, RunManifest, SchoolYear, UsJurisdiction};
use census_domain::model::CensusRun;
use census_store::Store;

#[path = "../../../../../../../tools/lineage_test_process.rs"]
mod process;

mod support {
    pub(super) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;
    pub(super) const FIXTURE_ROOT: &str = "CENSUS_LINEAGE_FIXTURE_ROOT";

    pub(super) fn fixture_dir() -> TestResult<tempfile::TempDir> {
        match std::env::var_os(FIXTURE_ROOT) {
            Some(root) => Ok(tempfile::Builder::new()
                .prefix("pinned-run-fence-")
                .tempdir_in(root)?),
            None => Ok(tempfile::tempdir()?),
        }
    }
}

use support::TestResult;

#[test]
fn initialized_unbound_fenced_view_derives_run_metadata_without_append_reentry() -> TestResult {
    process::bounded_case(
        concat!(
            module_path!(),
            "::initialized_unbound_fenced_view_derives_run_metadata_without_append_reentry"
        ),
        || {
            let directory = support::fixture_dir()?;
            let store = Store::open_with(directory.path(), 8_388_608)?;
            let identity = store.store_identity()?;
            let fence = store.fenced_snapshot();
            check!(eq; fence.view().run_manifest()?, None);
            let dataset = ExportDataset::from_snapshot(&store, fence.view())?;
            check!(eq; dataset.lineage.store_identity, identity.as_str());
            check!(eq; dataset.lineage.snapshot_sequence, fence.view().sequence());
            check!(eq; dataset.lineage.run, None);
            check!(eq; dataset.lineage.cohort, None);
            Ok(())
        },
    )
}

#[test]
fn initialized_bound_fenced_view_derives_run_metadata_without_append_reentry() -> TestResult {
    process::bounded_case(
        concat!(
            module_path!(),
            "::initialized_bound_fenced_view_derives_run_metadata_without_append_reentry"
        ),
        || {
            let directory = support::fixture_dir()?;
            let store = Store::open_with(directory.path(), 8_388_608)?;
            let identity = store.store_identity()?;
            let bound = RunManifest {
                store_identity: identity.as_str().to_string(),
                run: CensusRun::new(SchoolYear::new(2026).ok_or("season")?, 7).ok_or("run")?,
                cohort: GradYear::CO2027,
                jurisdictions: vec![
                    UsJurisdiction::Wisconsin,
                    UsJurisdiction::Illinois,
                    UsJurisdiction::Wisconsin,
                ],
            };
            store.bind_run(&bound)?;
            let fence = store.fenced_snapshot();
            check!(eq; fence.view().run_manifest()?, Some(bound.clone()));
            let dataset = ExportDataset::from_snapshot(&store, fence.view())?;
            check!(eq; dataset.lineage.store_identity, identity.as_str());
            check!(eq; dataset.lineage.snapshot_sequence, fence.view().sequence());
            check!(eq; dataset.lineage.run, Some(bound.run));
            check!(eq; dataset.lineage.cohort, Some(bound.cohort));
            Ok(())
        },
    )
}
