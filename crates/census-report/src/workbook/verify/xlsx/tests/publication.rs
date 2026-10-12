use super::TestResult;
use crate::workbook::publication::{current_workbook, verify_published, Stage};
use crate::workbook::verify::read::tests::publication;
use census_store::Store;
use std::path::Path;

#[test]
fn verify_corrupt_failed_publication_emits_no_certificate_and_keeps_the_prior_publication(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let (good, dataset, options) = publication(directory.path())?;
    let store = Store::open(directory.path().join("store"))?;
    let root = store.out_dir().join("publication");
    let generation = good.parent().ok_or("generation directory")?.to_path_buf();
    let certificate = std::fs::read(generation.join("manifest.json"))?;
    let pointer = std::fs::read_link(root.join("current"))?;
    let before = generations(&root)?;
    let censuses = crate::workbook::Censuses::of(&dataset, &store.out_dir());
    let stage = Stage::begin(&dataset, &store, &options)?;
    let result = stage.publish(&dataset, &options, |artifacts: &Path| {
        crate::workbook::write_artifacts(artifacts, &dataset, &options, &censuses)?;
        let workbook = artifacts.join("workbook.xlsx");
        let mut bytes = std::fs::read(&workbook)
            .map_err(|source| crate::report::io_error(&workbook, source))?;
        bytes.truncate(64);
        std::fs::write(&workbook, &bytes)
            .map_err(|source| crate::report::io_error(&workbook, source))?;
        Ok(())
    });
    check!(result.is_err(), "a truncated workbook was published");
    check!(eq; std::fs::read_link(root.join("current"))?, pointer, "the pointer did not move");
    check!(
        eq;
        std::fs::read(generation.join("manifest.json"))?,
        certificate,
        "the prior certificate changed"
    );
    check!(eq; generations(&root)?, before, "a failed publication installed a generation");
    check!(eq; current_workbook(&root)?, good);
    verify_published(&good)?;
    Ok(())
}

fn generations(root: &Path) -> TestResult<Vec<String>> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(root.join("generations"))? {
        let entry = entry?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }
    names.sort_unstable();
    Ok(names)
}
