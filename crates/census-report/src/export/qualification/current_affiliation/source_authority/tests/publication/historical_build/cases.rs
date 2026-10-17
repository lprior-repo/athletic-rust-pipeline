use super::*;
use authentication::{relative, string};

pub(super) fn replay(artifact: &authentication::Retained, retained: &Value) -> TestResult {
    let root = &artifact.root;
    let archive = relative(root, string(retained, "archive")?)?;
    let digest = string(retained, "archive_digest")?;
    let workbook = relative(root, string(retained, "workbook")?)?;
    let dataset = ExportDataset::reopen_frozen_with_digest(&archive, digest)?;
    check!(eq; dataset.archive_digest()?, digest);
    check!(eq; serde_json::to_value(&dataset.lineage)?, *retained.get("lineage").ok_or("retained lineage")?);
    check!(eq; dataset.lineage.store_identity, string(retained, "store_identity")?);
    let expected = expected(&dataset, retained)?;
    check!(eq; witness(&dataset, &expected)?, *retained.get("observations").ok_or("producer observations")?);
    inspect_wire(&archive)?;
    let publication = published::open_publication(
        &workbook,
        &dataset.lineage,
        Scope::AllSources,
        GradYear::CO2027,
    )?;
    check!(eq; witness(&publication.dataset, &expected)?, *retained.get("observations").ok_or("producer observations")?);
    check!(eq; artifacts::readback(&workbook, &publication.dataset, &expected)?, *retained.get("publication_rows").ok_or("producer readback")?);
    let verified = published::verify_published(&workbook)?;
    check!(eq; verified.generation_digest, string(retained, "publication_generation_digest")?);
    check!(eq; verified.workbook.mapped_athletes, u64::try_from(expected.selected_count())?);
    provenance::old_claim_refused_by_current(&archive, &artifact.manifest)?;
    Ok(())
}

fn expected(dataset: &ExportDataset, retained: &Value) -> TestResult<Expected> {
    let case = match string(retained, "case")? {
        "resolved" => Case::Resolved,
        "conflicted" => Case::Conflicted,
        "missing-owner" => Case::MissingOwner,
        _ => return Err("unsupported retained case".into()),
    };
    let athlete: AthleteId = serde_json::from_value(
        retained
            .get("expected_athlete")
            .ok_or("retained athlete")?
            .clone(),
    )?;
    let schools = dataset
        .schools
        .values()
        .filter(|school| {
            school.source_identities.iter().any(|source| {
                source.namespace == SourceNamespace::MilesplitSchool && source.id == "12439"
            })
        })
        .collect::<Vec<_>>();
    check!(eq; schools.len(), 1);
    let school = schools.first().ok_or("controlled retained first school")?;
    let expected = Expected {
        athlete,
        current_school: school.id.clone(),
        school_name: school.name.clone(),
        case,
    };
    check!(eq; expected.status(), string(retained, "expected_status")?);
    check!(eq; serde_json::to_value(expected.school())?, *retained.get("expected_school").ok_or("retained school result")?);
    check!(eq; retained.get("expected_selected_count").and_then(Value::as_u64), Some(u64::try_from(expected.selected_count())?));
    Ok(expected)
}
