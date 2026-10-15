use super::{change, population::Population, value, TestResult};
use crate::workbook::publication::{self, manifest};
use std::path::{Path, PathBuf};

#[test]
fn recaptured_current_hashes_cannot_hide_forged_result_group_counts_or_physical_claims(
) -> TestResult {
    let given = Population::current()?;
    let store = census_store::Store::open(given.directory.path().join("store"))?;
    let censuses = crate::workbook::Censuses::of(
        &given.dataset,
        &store.out_dir(),
        &given.options.jurisdictions,
    )?;
    let workbook = crate::workbook::build_from(&given.dataset, &store, &given.options, &censuses)?;
    let original = workbook.parent().ok_or("current generation")?;
    let accepted = publication::verify_published(&workbook)?;
    check!(eq; value(original, "best_marks.csv", 0, "eligible_result_group_count")?, "3");
    check!(eq; value(original, "best_marks.csv", 0, "tied_result_group_count")?, "2");
    check!(eq; value(original, "best_marks.csv", 0, "physical_performance_count_status")?, "unknown");
    for (index, (field, forged)) in [
        ("eligible_result_group_count", "999"),
        ("tied_result_group_count", "999"),
        ("tied_result_group_representative_ids", "[]"),
        ("result_group_basis", "matching_competition_context"),
        ("physical_performance_count_status", "exact"),
        ("selection_policy_version", "20"),
        ("supporting_tied_candidate_ids", "[]"),
    ]
    .into_iter()
    .enumerate()
    {
        let changed = copy(original, given.directory.path(), index)?;
        change(&changed, "best_marks.csv", field, forged)?;
        recapture(&changed, &given)?;
        manifest::verify_directory(&changed)?;
        check!(
            manifest::verify_prepared(&changed, &given.dataset, &given.options).is_err(),
            "forged {field}"
        );
        check!(
            publication::verify_published(&changed.join("workbook.xlsx")).is_err(),
            "forged {field}"
        );
    }
    check!(eq; publication::verify_published(&workbook)?.generation_digest, accepted.generation_digest);
    Ok(())
}

#[test]
fn current_flat_sidecars_refuse_legacy_count_labels_after_hash_recapture() -> TestResult {
    let given = Population::current()?;
    let store = census_store::Store::open(given.directory.path().join("store"))?;
    let censuses = crate::workbook::Censuses::of(
        &given.dataset,
        &store.out_dir(),
        &given.options.jurisdictions,
    )?;
    let workbook = crate::workbook::build_from(&given.dataset, &store, &given.options, &censuses)?;
    let original = workbook.parent().ok_or("current generation")?;
    for (index, extension) in ["csv", "jsonl"].into_iter().enumerate() {
        let changed = copy(original, given.directory.path(), index)?;
        let path = changed.join(format!("best-results-co2027.{extension}"));
        let bytes = std::fs::read_to_string(&path)?;
        check!(bytes.contains("tied_result_group_count"));
        std::fs::write(
            &path,
            bytes.replace("tied_result_group_count", "tied_performance_count"),
        )?;
        recapture(&changed, &given)?;
        manifest::verify_directory(&changed)?;
        check!(
            publication::verify_published(&changed.join("workbook.xlsx")).is_err(),
            "legacy {extension} label"
        );
    }
    publication::verify_published(&workbook)?;
    Ok(())
}

fn copy(original: &Path, root: &Path, index: usize) -> TestResult<PathBuf> {
    let changed = root.join(format!("result-group-mutation-{index}"));
    std::fs::create_dir(&changed)?;
    for entry in std::fs::read_dir(original)? {
        let entry = entry?;
        std::fs::copy(entry.path(), changed.join(entry.file_name()))?;
    }
    Ok(changed)
}

fn recapture(directory: &Path, given: &Population) -> TestResult {
    let captured = manifest::Manifest::recapture(directory, &given.dataset, &given.options)?;
    std::fs::write(
        directory.join("manifest.json"),
        serde_json::to_vec(&captured)?,
    )?;
    Ok(())
}
