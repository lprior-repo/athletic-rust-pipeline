use super::super::{population::Population, read, TestResult};
use crate::workbook::publication::{self, manifest};
use std::path::{Path, PathBuf};

#[test]
fn artifact_v6_recaptured_hashes_do_not_hide_missing_or_forged_supporting_references() -> TestResult
{
    let given = Population::current()?;
    let store = census_store::Store::open(given.directory.path().join("store"))?;
    let censuses = crate::workbook::Censuses::of(
        &given.dataset,
        &store.out_dir(),
        &given.options.jurisdictions,
    )?;
    let workbook = crate::workbook::build_from(&given.dataset, &store, &given.options, &censuses)?;
    let original = workbook.parent().ok_or("missing published generation")?;
    let accepted = publication::verify_published(&workbook)?;
    let before = std::fs::read(original.join("provenance.csv"))?;
    let manifest_json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(original.join("manifest.json"))?)?;
    check!(eq; manifest_json.get("schema_revision"), Some(&serde_json::json!(6)));
    for mutation in ["missing", "forged", "extra", "ids"] {
        let changed = copy(original, given.directory.path(), mutation)?;
        mutate(&changed, mutation)?;
        let recaptured = manifest::Manifest::recapture(&changed, &given.dataset, &given.options)?;
        std::fs::remove_file(changed.join("manifest.json"))?;
        publication::write_json(&changed.join("manifest.json"), &recaptured)?;
        manifest::verify_directory(&changed)?;
        check!(manifest::verify_prepared(&changed, &given.dataset, &given.options).is_err());
        check!(publication::verify_published(&changed.join("workbook.xlsx")).is_err());
    }
    check!(eq; std::fs::read(original.join("provenance.csv"))?, before);
    check!(eq; publication::verify_published(&workbook)?.generation_digest, accepted.generation_digest);
    Ok(())
}

fn copy(original: &Path, root: &Path, name: &str) -> TestResult<PathBuf> {
    let copied = root.join(format!("v4-tampered-{name}"));
    std::fs::create_dir(&copied)?;
    for entry in std::fs::read_dir(original)? {
        let entry = entry?;
        std::fs::copy(entry.path(), copied.join(entry.file_name()))?;
    }
    Ok(copied)
}

fn mutate(directory: &Path, mutation: &str) -> TestResult {
    if mutation == "ids" {
        return super::super::change(
            directory,
            "best_marks.csv",
            "supporting_tied_candidate_ids",
            "[]",
        );
    }
    let (header, mut rows) = read(directory, "provenance.csv")?;
    let kind = super::fixture::column(&header, "evidence_type")?;
    let index = rows
        .iter()
        .position(|row| row.get(kind).map(String::as_str) == Some("retained_performance_reference"))
        .ok_or("missing produced reference")?;
    match mutation {
        "missing" => {
            rows.remove(index);
        }
        "forged" => {
            let field = super::fixture::column(&header, "retained_fact")?;
            *rows
                .get_mut(index)
                .and_then(|row| row.get_mut(field))
                .ok_or("missing reference fact")? = "{}".into();
        }
        "extra" => {
            let mut extra = rows
                .get(index)
                .ok_or("missing extra reference base")?
                .clone();
            *extra.first_mut().ok_or("missing evidence id")? = "unretained-reference".into();
            rows.push(extra);
        }
        _ => return Err("unknown reference mutation".into()),
    }
    super::fixture::put(directory, "provenance.csv", &header, &rows)
}
