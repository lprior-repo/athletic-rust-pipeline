use super::{publication, ExportDataset, Fixture, Scope, TestResult, Value};
use crate::workbook::{Censuses, Options};
use census_domain::model::{serialized_digest, ContractRefV1, ProfileDate, ProfileId, SchoolYear};
use census_domain::UsJurisdiction;
use serde_json::json;
use std::path::Path;

#[derive(serde::Deserialize, serde::Serialize)]
struct Selection {
    scope: String,
    grad_year: Option<i16>,
    limit: Option<usize>,
    school_year: Option<SchoolYear>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    jurisdictions: Vec<String>,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct SelectedBinding {
    profile_id: ProfileId,
    as_of: ProfileDate,
    participation_rule: ContractRefV1,
}

pub(super) fn reject_option_overrides(
    fixture: &Fixture,
    dataset: &ExportDataset,
    directory: &Path,
) -> TestResult {
    let base = Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: Scope::AllSources,
        jurisdictions: vec![UsJurisdiction::Alabama, UsJurisdiction::Wisconsin],
        school_year: SchoolYear::new(2026).ok_or("contact year")?,
    };
    let cases = [
        (
            Options {
                school_year: SchoolYear::new(2025).ok_or("different year")?,
                ..base.clone()
            },
            "contact school year",
        ),
        (
            Options {
                grad_year: None,
                ..base.clone()
            },
            "graduation year",
        ),
        (
            Options {
                grad_year: Some(2028),
                ..base.clone()
            },
            "graduation year",
        ),
        (
            Options {
                jurisdictions: Vec::new(),
                ..base.clone()
            },
            "jurisdictions",
        ),
        (
            Options {
                jurisdictions: vec![UsJurisdiction::Alabama],
                ..base.clone()
            },
            "jurisdictions",
        ),
        (
            Options {
                jurisdictions: vec![
                    UsJurisdiction::Alabama,
                    UsJurisdiction::Michigan,
                    UsJurisdiction::Wisconsin,
                ],
                ..base.clone()
            },
            "jurisdictions",
        ),
        (
            Options {
                jurisdictions: vec![UsJurisdiction::Wisconsin, UsJurisdiction::Alabama],
                ..base.clone()
            },
            "canonical profile order",
        ),
        (
            Options {
                scope: Scope::Core,
                ..base.clone()
            },
            "all-source evidence scope",
        ),
        (
            Options {
                limit: Some(0),
                ..base
            },
            "limit the profile population",
        ),
    ];
    let censuses = Censuses::of(
        dataset,
        &fixture.store.out_dir(),
        &[UsJurisdiction::Alabama, UsJurisdiction::Wisconsin],
    )?;
    for (index, (mut options, expected)) in cases.into_iter().enumerate() {
        let root = directory.join(format!("invalid-options-{index}"));
        options.out = Some(root.clone());
        let sequence = fixture.store.snapshot().sequence();
        let error =
            crate::workbook::verify::verify_frozen(&root.join("absent.xlsx"), dataset, &options)
                .err()
                .ok_or("standalone verifier accepted profile option override")?;
        check!(error.to_string().contains(expected));
        for existing in [false, true] {
            if existing {
                std::fs::create_dir(&root)?;
                std::fs::write(root.join("sentinel"), b"unchanged existing root")?;
            }
            let error = crate::workbook::build_from(dataset, &fixture.store, &options, &censuses)
                .err()
                .ok_or("profile option override accepted")?;
            check!(error.to_string().contains(expected));
            check!(eq; fixture.store.snapshot().sequence(), sequence);
            if existing {
                check!(eq; std::fs::read(root.join("sentinel"))?, b"unchanged existing root");
                let entries = std::fs::read_dir(&root)?.collect::<Result<Vec<_>, _>>()?;
                check!(eq; entries.len(), 1);
            } else {
                check!(!root.exists());
            }
        }
    }
    Ok(())
}

pub(super) fn manifest_revision(directory: &Path) -> TestResult {
    let manifest: Value = serde_json::from_slice(&std::fs::read(directory.join("manifest.json"))?)?;
    check!(eq; manifest.get("schema_revision"), Some(&json!(11)));
    check!(eq; manifest.pointer("/selection/school_year"), Some(&json!(2026)));
    Ok(())
}

pub(super) fn reject_rehashed_contact_year_override(workbook: &Path) -> TestResult {
    let path = workbook.parent().ok_or("generation")?.join("manifest.json");
    let original = std::fs::read(&path)?;
    let mut manifest: Value = serde_json::from_slice(&original)?;
    *manifest
        .pointer_mut("/selection/school_year")
        .ok_or("manifest contact year")? = json!(2025);
    let lineage: crate::export::DatasetLineage =
        serde_json::from_value(manifest.get("lineage").ok_or("lineage")?.clone())?;
    let selection: Selection =
        serde_json::from_value(manifest.get("selection").ok_or("selection")?.clone())?;
    let selected: SelectedBinding = serde_json::from_value(
        manifest
            .get("selected_profile")
            .ok_or("selected profile")?
            .clone(),
    )?;
    let digest = serialized_digest(&(
        manifest.get("schema_revision").ok_or("revision")?,
        lineage,
        selection,
        manifest.get("artifacts").ok_or("artifacts")?,
        manifest.get("normalized_csv_rows").ok_or("counts")?,
        selected,
    ))?;
    *manifest
        .get_mut("generation_digest")
        .ok_or("generation digest")? = json!(digest);
    std::fs::write(&path, serde_json::to_vec(&manifest)?)?;
    let result = publication::verify_published(workbook);
    std::fs::write(&path, original)?;
    let error = result
        .err()
        .ok_or("rehashed contact year override accepted")?;
    check!(error
        .to_string()
        .contains("contact school year cannot be overridden"));
    publication::verify_published(workbook)?;
    Ok(())
}
