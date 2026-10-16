use super::super::historical_fixture;
use super::{options, seed, Manifest, Store, TestResult};
use crate::workbook::publication;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const FAMILIES: [&str; 9] = [
    "athletes.csv",
    "best_marks.csv",
    "schools.csv",
    "coach_contacts.csv",
    "comparison_classes.csv",
    "provenance.csv",
    "coverage.csv",
    "identity_aliases.csv",
    "combined_exclusions.csv",
];

fn manifest(directory: &Path) -> TestResult<Value> {
    Ok(serde_json::from_slice(&std::fs::read(
        directory.join("manifest.json"),
    )?)?)
}

fn counts(value: &Value) -> TestResult<BTreeMap<String, u64>> {
    Ok(serde_json::from_value(
        value
            .get("normalized_csv_rows")
            .ok_or("missing normalized row counts")?
            .clone(),
    )?)
}

fn expected(populated: bool) -> BTreeMap<String, u64> {
    FAMILIES
        .into_iter()
        .map(|name| {
            let count = match (populated, name) {
                (true, "athletes.csv" | "schools.csv" | "identity_aliases.csv") => 1,
                (true, "provenance.csv") => 3,
                _ => 0,
            };
            (name.into(), count)
        })
        .collect()
}

fn data_records(directory: &Path, name: &str) -> TestResult<u64> {
    csv::Reader::from_path(directory.join(name))?
        .records()
        .try_fold(0_u64, |count, record| {
            record?;
            count
                .checked_add(1)
                .ok_or_else(|| "test CSV count overflow".into())
        })
}

fn copied(source: &Path, root: &Path, name: &str) -> TestResult<PathBuf> {
    let destination = root.join(name);
    std::fs::create_dir(&destination)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        std::fs::copy(entry.path(), destination.join(entry.file_name()))?;
    }
    Ok(destination)
}

fn bytes(directory: &Path) -> TestResult<BTreeMap<String, Vec<u8>>> {
    std::fs::read_dir(directory)?
        .map(|entry| {
            let entry = entry?;
            Ok((
                entry.file_name().to_string_lossy().into_owned(),
                std::fs::read(entry.path())?,
            ))
        })
        .collect()
}

fn rehash(directory: &Path, mut value: Value) -> TestResult {
    let raw_counts = value
        .as_object_mut()
        .ok_or("manifest fixture object")?
        .remove("normalized_csv_rows");
    let mut value: Manifest = serde_json::from_value(value)?;
    value.normalized_csv_rows = raw_counts.map(serde_json::from_value).transpose()?;
    check!(eq; value.schema_revision, publication::ARTIFACT_REVISION);
    value.generation_digest = census_domain::model::serialized_digest(&(
        value.schema_revision,
        &value.lineage,
        &value.selection,
        &value.artifacts,
        &value.normalized_csv_rows,
    ))?;
    std::fs::write(directory.join("manifest.json"), serde_json::to_vec(&value)?)?;
    Ok(())
}

#[test]
fn current_manifest_records_every_normalized_family_even_when_all_are_empty() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    let workbook = crate::workbook::build(&store, &options()?)?;
    let directory = workbook.parent().ok_or("missing generation")?;
    let value = manifest(directory)?;
    check!(eq; value.get("schema_revision"), Some(&json!(publication::ARTIFACT_REVISION)));
    check!(eq; counts(&value)?, expected(false));
    for name in FAMILIES {
        check!(eq; data_records(directory, name)?, 0);
    }
    publication::verify_published(&workbook)?;
    Ok(())
}

#[test]
fn current_manifest_counts_records_rather_than_embedded_newlines_or_headers() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    seed(&store, "Count, \"Quoted\"\nSchool")?;
    let workbook = crate::workbook::build(&store, &options()?)?;
    let directory = workbook.parent().ok_or("missing generation")?;
    let value = manifest(directory)?;
    let actual = counts(&value)?;
    check!(eq; actual, expected(true));
    for name in FAMILIES {
        check!(eq; actual.get(name), Some(&data_records(directory, name)?));
    }
    let school = std::fs::read_to_string(directory.join("schools.csv"))?;
    check!(school.lines().count() > 2);
    publication::verify_published(&workbook)?;
    Ok(())
}

#[test]
fn current_manifest_refuses_missing_extra_wrong_and_maximum_counts_after_digest_recapture(
) -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    seed(&store, "Count Mutation High")?;
    let workbook = crate::workbook::build(&store, &options()?)?;
    let original = workbook.parent().ok_or("missing generation")?;
    let before = bytes(original)?;
    for mutation in ["missing", "extra", "wrong", "maximum"] {
        let directory = copied(original, temporary.path(), mutation)?;
        let mut value = manifest(&directory)?;
        let map = value
            .get_mut("normalized_csv_rows")
            .and_then(Value::as_object_mut)
            .ok_or("missing count object")?;
        match mutation {
            "missing" => {
                map.remove("athletes.csv");
            }
            "extra" => {
                map.insert("unlisted.csv".into(), json!(0));
            }
            "wrong" => {
                map.insert("athletes.csv".into(), json!(0));
            }
            "maximum" => {
                map.insert("athletes.csv".into(), json!(u64::MAX));
            }
            _ => return Err("unknown count mutation".into()),
        }
        rehash(&directory, value)?;
        if mutation == "extra" {
            let raw = std::fs::read(directory.join("manifest.json"))?;
            let error = serde_json::from_slice::<Manifest>(&raw)
                .err()
                .ok_or("extra family decoded")?;
            check!(error.to_string().contains("exceeds nine entries"));
        }
        if matches!(mutation, "wrong" | "maximum") {
            super::super::verify_directory(&directory)?;
        }
        check!(
            publication::verify_published(&directory.join("workbook.xlsx")).is_err(),
            "accepted {mutation} counts"
        );
    }
    check!(eq; bytes(original)?, before);
    publication::verify_published(&workbook)?;
    Ok(())
}

#[test]
fn current_manifest_refuses_absent_null_noninteger_and_overflow_count_encodings() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    let workbook = crate::workbook::build(&store, &options()?)?;
    let original = workbook.parent().ok_or("missing generation")?;
    for mutation in ["absent", "null", "negative", "fraction", "overflow"] {
        let directory = copied(original, temporary.path(), mutation)?;
        let mut value = manifest(&directory)?;
        if mutation == "absent" {
            value
                .as_object_mut()
                .ok_or("manifest object")?
                .remove("normalized_csv_rows");
        } else if mutation == "null" {
            value["normalized_csv_rows"] = Value::Null;
        } else {
            value["normalized_csv_rows"]["athletes.csv"] = match mutation {
                "negative" => json!(-1),
                "fraction" => json!(0.5),
                "overflow" => json!("OVERFLOW_COUNT_SENTINEL"),
                _ => return Err("unknown count encoding".into()),
            };
        }
        if mutation == "absent" {
            rehash(&directory, value)?;
        } else {
            let encoded = serde_json::to_string(&value)?
                .replace("\"OVERFLOW_COUNT_SENTINEL\"", "18446744073709551616");
            check!(
                serde_json::from_str::<Manifest>(&encoded).is_err(),
                "decoded {mutation} count encoding"
            );
            std::fs::write(directory.join("manifest.json"), encoded)?;
        }
        check!(
            publication::verify_published(&directory.join("workbook.xlsx")).is_err(),
            "accepted {mutation} encoding"
        );
    }
    publication::verify_published(&workbook)?;
    Ok(())
}

#[test]
fn recaptured_csv_hash_and_row_count_cannot_hide_an_extra_source_unbacked_row() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    seed(&store, "Forged Count High")?;
    let workbook = crate::workbook::build(&store, &options()?)?;
    let original = workbook.parent().ok_or("missing generation")?;
    let before = bytes(original)?;
    let directory = copied(original, temporary.path(), "forged-row")?;
    let path = directory.join("athletes.csv");
    let mut reader = csv::Reader::from_path(&path)?;
    let header = reader.headers()?.clone();
    let records = reader.records().collect::<Result<Vec<_>, _>>()?;
    let first = records.first().ok_or("missing original athlete")?;
    let mut forged = first.iter().map(str::to_owned).collect::<Vec<_>>();
    *forged.first_mut().ok_or("athlete key")? = "unretained-athlete".into();
    drop(reader);
    let mut writer = csv::Writer::from_path(&path)?;
    writer.write_record(&header)?;
    for record in &records {
        writer.write_record(record)?;
    }
    writer.write_record(&forged)?;
    writer.flush()?;
    drop(writer);
    let raw = std::fs::read(&path)?;
    super::repoint_artifact(&directory, "athletes.csv", &raw)?;
    let mut value = manifest(&directory)?;
    value["normalized_csv_rows"]["athletes.csv"] = json!(2);
    rehash(&directory, value)?;
    super::super::verify_directory(&directory)?;
    check!(publication::verify_published(&directory.join("workbook.xlsx")).is_err());
    check!(eq; bytes(original)?, before);
    publication::verify_published(&workbook)?;
    Ok(())
}

#[test]
fn obsolete_policy20_v4_reconstruction_refuses_before_mutation() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    seed(&store, "Historical V4 Count High")?;
    let workbook = crate::workbook::build(&store, &options()?)?;
    let directory = temporary.path().join("reconstructed-policy20-v4");
    let error = historical_fixture::render_policy20(&store, &directory, &options()?, 4)
        .err()
        .ok_or("obsolete policy-20 v4 reconstruction was accepted")?;
    check!(error
        .to_string()
        .contains("incompatible with selection policy 20"));
    check!(!directory.join("manifest.json").exists());
    check!(!directory.join("workbook.xlsx").exists());
    publication::verify_published(&workbook)?;
    Ok(())
}

#[test]
fn obsolete_policy20_v5_reconstruction_refuses_before_mutation() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    seed(&store, "Historical V5 Count High")?;
    let directory = temporary.path().join("reconstructed-policy20-v5");
    let error = historical_fixture::render_policy20(&store, &directory, &options()?, 5)
        .err()
        .ok_or("obsolete policy-20 v5 reconstruction was accepted")?;
    check!(error
        .to_string()
        .contains("incompatible with selection policy 20"));
    check!(!directory.join("manifest.json").exists());
    check!(!directory.join("workbook.xlsx").exists());
    Ok(())
}
