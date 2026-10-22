use super::{
    publication, BundleBytes, Case, DatasetLineage, GradYear, Path, PathBuf, Scope, TestResult,
    Value,
};

pub(super) fn selected_bundle(
    path: &Path,
    lineage: &DatasetLineage,
    witness: &Value,
) -> TestResult {
    let verified = publication::verify_published(path)?;
    check!(eq; verified.workbook.mapped_athletes, 1);
    let opened = publication::open_publication(path, lineage, Scope::AllSources, GradYear::CO2027)?;
    check!(eq; opened.generation_digest, verified.generation_digest);
    check!(eq; super::super::super::witness::capture(&opened.dataset)?, *witness);
    let generation = path.parent().ok_or("selected generation")?;
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(generation.join("manifest.json"))?)?;
    check!(eq; manifest.get("schema_revision").and_then(Value::as_u64), Some(11));
    check!(manifest.get("selected_profile").is_some());
    let counts = manifest
        .get("normalized_csv_rows")
        .and_then(Value::as_object)
        .ok_or("selected normalized counts")?;
    check!(eq; counts.len(), 11);
    for name in [
        "athletes.csv",
        "best_marks.csv",
        "schools.csv",
        "comparison_classes.csv",
        "coach_contacts.csv",
        "combined_exclusions.csv",
        "coverage.csv",
        "provenance.csv",
        "identity_aliases.csv",
        "current_affiliations.csv",
        "participation_exclusions.csv",
    ] {
        let rows = csv::Reader::from_path(generation.join(name))?
            .records()
            .collect::<Result<Vec<_>, _>>()?;
        check!(eq; counts.get(name).and_then(Value::as_u64), Some(u64::try_from(rows.len())?));
    }
    check!(eq; counts.get("athletes.csv").and_then(Value::as_u64), Some(1));
    check!(eq; counts.get("best_marks.csv").and_then(Value::as_u64), Some(0));
    check!(eq; counts.get("participation_exclusions.csv").and_then(Value::as_u64), Some(u64::try_from(opened.dataset.performances.len())?));
    for performance in &opened.dataset.performances {
        let assessment = opened
            .dataset
            .verified_participation()?
            .assessment(&performance.id)
            .ok_or("recovered participation")?;
        check!(eq; assessment.outcome(), crate::export::ParticipationOutcome::Unresolved);
        check!(eq; assessment.reasons().iter().collect::<Vec<_>>(), vec![
            census_domain::participation_candidates::ParticipationReason::HighSchoolStatusMissing,
            census_domain::participation_candidates::ParticipationReason::SummerBoundaryUnresolved,
        ]);
        check!(assessment.diagnostics().is_empty());
    }
    let frozen: Value =
        serde_json::from_slice(&std::fs::read(generation.join("frozen-input.json"))?)?;
    check!(frozen.get("selected_proofs").is_some());
    check!(
        std::fs::read_dir(generation.join("frozen-input.json.proofs"))?
            .next()
            .transpose()?
            .is_some()
    );
    Ok(())
}

pub(super) fn bundle_bytes(directory: &Path) -> TestResult<BundleBytes> {
    let mut bytes = BundleBytes::new();
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let name = PathBuf::from(entry.file_name());
        let kind = entry.file_type()?;
        if kind.is_dir() {
            check!(eq; name.as_path(), Path::new("frozen-input.json.proofs"));
            for object in std::fs::read_dir(entry.path())? {
                let object = object?;
                check!(object.file_type()?.is_file());
                check!(eq; bytes.insert(name.join(object.file_name()), std::fs::read(object.path())?), None);
            }
        } else {
            check!(kind.is_file());
            check!(eq; bytes.insert(name, std::fs::read(entry.path())?), None);
        }
    }
    Ok(bytes)
}

pub(super) fn old_unchanged(case: &Case) -> TestResult {
    check!(eq; bundle_bytes(case.old.parent().ok_or("old generation")?)?, case.old_bytes);
    selected_bundle(&case.old, &case.old_lineage, &case.old_witness)
}
