use super::{files, population::Population, TestResult};
use crate::report::{ReportError, ReportResult};
use crate::workbook::publication::{self, manifest};
use std::path::{Path, PathBuf};

#[test]
fn given_exact_frozen_generation_when_every_family_is_read_then_all_generation_cells_match(
) -> TestResult {
    let given = Population::new()?;
    let directory = tempfile::tempdir()?;
    files::write(&given, directory.path())?;
    for family in super::FAMILIES {
        let (header, records) = super::read(directory.path(), family)?;
        let column = header
            .iter()
            .position(|name| name == "input_generation")
            .ok_or("missing generation column")?;
        check!(!records.is_empty(), "{family} must exercise a data record");
        for record in records {
            check!(eq; record.get(column), Some(&given.dataset.lineage.input_generation));
        }
    }
    super::super::verify(directory.path(), &given.inputs()?)?;
    Ok(())
}

#[test]
fn given_foreign_generation_in_any_family_when_read_then_semantic_generation_binding_refuses(
) -> TestResult {
    let given = Population::new()?;
    let directory = tempfile::tempdir()?;
    for family in super::FAMILIES {
        files::write(&given, directory.path())?;
        super::super::verify(directory.path(), &given.inputs()?)?;
        super::change(
            directory.path(),
            family,
            "input_generation",
            "foreign-generation",
        )?;
        super::rejected(
            super::super::verify(directory.path(), &given.inputs()?),
            family,
            "foreign-generation",
        )?;
    }
    Ok(())
}

#[test]
fn given_missing_or_unavailable_alias_generation_when_read_then_exact_frozen_binding_is_required(
) -> TestResult {
    let given = Population::new()?;
    let directory = tempfile::tempdir()?;
    for replacement in ["", "unavailable", "foreign-generation"] {
        files::write(&given, directory.path())?;
        super::super::verify(directory.path(), &given.inputs()?)?;
        super::change(
            directory.path(),
            "identity_aliases.csv",
            "input_generation",
            replacement,
        )?;
        alias_refused(
            super::super::verify(directory.path(), &given.inputs()?),
            directory.path(),
            &given.dataset.lineage.input_generation,
            replacement,
        )?;
    }
    Ok(())
}

#[test]
fn given_recaptured_manifest_hashes_when_alias_generation_changes_then_both_readback_paths_refuse(
) -> TestResult {
    let given = Population::new()?;
    let store = census_store::Store::open(given.directory.path().join("store"))?;
    let censuses = crate::workbook::Censuses::of(
        &given.dataset,
        &store.out_dir(),
        &given.options.jurisdictions,
    );
    let workbook = crate::workbook::build_from(&given.dataset, &store, &given.options, &censuses)?;
    let original = workbook.parent().ok_or("missing generated directory")?;
    let accepted = publication::verify_published(&workbook)?;
    manifest::verify_prepared(original, &given.dataset, &given.options)?;
    let retained = std::fs::read(original.join("identity_aliases.csv"))?;
    let changed = copy_generation(original, given.directory.path())?;
    super::change(
        &changed,
        "identity_aliases.csv",
        "input_generation",
        "foreign-generation",
    )?;
    let recaptured = manifest::Manifest::capture(&changed, &given.dataset, &given.options)?;
    check!(ne; recaptured.generation_digest, accepted.generation_digest);
    std::fs::remove_file(changed.join("manifest.json"))?;
    publication::write_json(&changed.join("manifest.json"), &recaptured)?;
    let verified_hashes = manifest::verify_directory(&changed)?;
    check!(eq; verified_hashes.generation_digest, recaptured.generation_digest);
    alias_refused(
        manifest::verify_prepared(&changed, &given.dataset, &given.options),
        &changed,
        &given.dataset.lineage.input_generation,
        "foreign-generation",
    )?;
    alias_refused(
        publication::verify_published(&changed.join("workbook.xlsx")).map(|_| ()),
        &changed,
        &given.dataset.lineage.input_generation,
        "foreign-generation",
    )?;
    check!(eq; std::fs::read(original.join("identity_aliases.csv"))?, retained);
    check!(eq; publication::verify_published(&workbook)?.generation_digest, accepted.generation_digest);
    Ok(())
}

fn alias_refused(
    result: ReportResult<()>,
    directory: &Path,
    expected: &str,
    found: &str,
) -> TestResult {
    match result {
        Err(ReportError::Invariant { detail }) => check!(eq; detail, format!(
            "{} record 2 column 7: expected {expected:?}, found {found:?}",
            directory.join("identity_aliases.csv").display()
        )),
        other => return Err(format!("alias generation was not refused: {other:?}").into()),
    }
    Ok(())
}

fn copy_generation(original: &Path, root: &Path) -> TestResult<PathBuf> {
    let copied = root.join("recaptured-generation");
    std::fs::create_dir(&copied)?;
    for entry in std::fs::read_dir(original)? {
        let entry = entry?;
        std::fs::copy(entry.path(), copied.join(entry.file_name()))?;
    }
    Ok(copied)
}
