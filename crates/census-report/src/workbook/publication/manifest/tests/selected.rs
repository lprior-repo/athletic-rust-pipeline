use super::super::{artifact_names, Manifest};
use super::TestResult;
use crate::workbook::publication::{
    inventory,
    selected::{fixture::Fixture, SelectedProfile},
};
use serde_json::{json, Value};
use std::path::Path;

fn historical_wire(directory: &Path, fixture: &Fixture) -> TestResult<Manifest> {
    let counts = inventory::names(7)?
        .iter()
        .map(|name| ((*name).into(), 1))
        .collect();
    let mut artifacts = std::collections::BTreeMap::new();
    for name in artifact_names(7, fixture.options.grad_year)? {
        let path = directory.join(&name);
        std::fs::write(&path, b"synthetic structural manifest fixture")?;
        artifacts.insert(name, super::super::hash_artifact(&path)?);
    }
    let mut manifest = Manifest {
        schema_revision: 7,
        lineage: fixture.dataset.lineage.clone(),
        selection: super::super::Selection::of(&fixture.options),
        artifacts,
        normalized_csv_rows: Some(counts),
        selected_profile: Some(SelectedProfile::of(&fixture.dataset)?),
        generation_digest: String::new(),
    };
    manifest.generation_digest = manifest.digest()?;
    Ok(manifest)
}

#[test]
fn historical_selected_wire_decodes_eleven_counts_and_preserves_revision_six_inventory(
) -> TestResult {
    let fixture = Fixture::new()?;
    let directory = tempfile::tempdir()?;
    let manifest = historical_wire(directory.path(), &fixture)?;
    check!(eq; manifest.schema_revision, 7);
    check!(eq; manifest.lineage.policy_revision, 21);
    let counts = manifest.normalized_csv_rows.as_ref().ok_or("counts")?;
    check!(eq; counts.len(), 11);
    check!(eq; counts.get("participation_exclusions.csv"), Some(&1));
    check!(eq; counts.get("current_affiliations.csv"), Some(&1));
    check!(manifest
        .artifacts
        .contains_key("participation_exclusions.csv"));
    check!(manifest.selected_profile.is_some());
    check!(manifest.ensure_dataset(&fixture.dataset).is_err());
    let encoded = serde_json::to_vec(&manifest)?;
    let decoded: Manifest = serde_json::from_slice(&encoded)?;
    check!(eq; decoded.digest()?, manifest.generation_digest);
    check!(eq; decoded.recipe()?, manifest.recipe()?);
    std::fs::write(directory.path().join("manifest.json"), &encoded)?;
    check!(super::super::verify_directory(directory.path()).is_err());
    check!(eq; inventory::names(6)?.len(), 9);
    check!(!artifact_names(6, fixture.options.grad_year)?
        .iter()
        .any(|name| name == "participation_exclusions.csv"));
    Ok(())
}

#[test]
fn selected_manifest_requires_complete_profile_binding_and_refuses_foreign_binding() -> TestResult {
    let fixture = Fixture::new()?;
    let directory = tempfile::tempdir()?;
    let manifest = historical_wire(directory.path(), &fixture)?;
    let original = serde_json::to_value(&manifest)?;
    let mut null = original.clone();
    null["selected_profile"] = Value::Null;
    check!(serde_json::from_value::<Manifest>(null).is_err());
    for field in [
        "selected_profile",
        "profile_id",
        "as_of",
        "participation_rule",
    ] {
        let mut value = original.clone();
        if field == "selected_profile" {
            value.as_object_mut().ok_or("manifest")?.remove(field);
        } else {
            value
                .get_mut("selected_profile")
                .and_then(Value::as_object_mut)
                .ok_or("profile")?
                .remove(field);
        }
        check!(serde_json::from_value::<Manifest>(value).is_err());
    }
    let mut value = original;
    value["selected_profile"]["profile_id"] = json!("c".repeat(64));
    let mut changed: Manifest = serde_json::from_value(value)?;
    check!(changed.ensure_dataset(&fixture.dataset).is_err());
    check!(ne; changed.recipe()?, manifest.recipe()?);
    changed.generation_digest = changed.digest()?;
    std::fs::write(
        directory.path().join("manifest.json"),
        serde_json::to_vec(&changed)?,
    )?;
    check!(super::super::verify_directory(directory.path()).is_err());
    check!(
        super::super::verify_prepared(directory.path(), &fixture.dataset, &fixture.options)
            .is_err()
    );
    Ok(())
}

#[test]
fn historical_manifest_decode_keeps_nine_entry_limit_and_rejects_selected_metadata() -> TestResult {
    let fixture = Fixture::new()?;
    let directory = tempfile::tempdir()?;
    let manifest = historical_wire(directory.path(), &fixture)?;
    let original = serde_json::to_value(&manifest)?;
    for revision in 1..=6 {
        let mut value = original.clone();
        value["schema_revision"] = json!(revision);
        value
            .as_object_mut()
            .ok_or("manifest")?
            .remove("selected_profile");
        let error = serde_json::from_value::<Manifest>(value.clone())
            .err()
            .ok_or("selected legacy counts accepted")?;
        check!(error.to_string().contains("exceeds nine entries"));
        value["normalized_csv_rows"]
            .as_object_mut()
            .ok_or("counts")?
            .remove("participation_exclusions.csv");
        value["normalized_csv_rows"]
            .as_object_mut()
            .ok_or("counts")?
            .remove("current_affiliations.csv");
        let decoded: Manifest = serde_json::from_value(value.clone())?;
        check!(decoded.selected_profile.is_none());
        value["selected_profile"] = Value::Null;
        check!(serde_json::from_value::<Manifest>(value.clone()).is_err());
        value["selected_profile"] = original["selected_profile"].clone();
        check!(serde_json::from_value::<Manifest>(value).is_err());
    }
    Ok(())
}

#[test]
fn selected_manifest_capture_refuses_frozen_json_without_runtime_verified_views() -> TestResult {
    let fixture = Fixture::new()?;
    let directory = tempfile::tempdir()?;
    let frozen = directory.path().join("frozen.json");
    fixture.dataset.save_frozen(&frozen)?;
    let reopened = crate::export::ExportDataset::reopen_frozen(&frozen)?;
    let counts = inventory::names(7)?
        .iter()
        .map(|name| ((*name).into(), 0))
        .collect();
    let absent = directory.path().join("not-created");
    check!(Manifest::capture(&absent, &reopened, &fixture.options, counts).is_err());
    check!(!absent.exists());
    Ok(())
}

#[test]
fn current_capture_refuses_unqualified_synthetic_selected_views() -> TestResult {
    let fixture = Fixture::new()?;
    let directory = tempfile::tempdir()?;
    let absent = directory.path().join("not-created");
    let counts = inventory::names(8)?
        .iter()
        .map(|name| ((*name).into(), 0))
        .collect();
    let error = Manifest::capture(&absent, &fixture.dataset, &fixture.options, counts)
        .err()
        .ok_or("unqualified synthetic views authorized current capture")?;
    check!(error
        .to_string()
        .contains("selected input has no captured authority binding"));
    check!(!absent.exists());
    Ok(())
}
