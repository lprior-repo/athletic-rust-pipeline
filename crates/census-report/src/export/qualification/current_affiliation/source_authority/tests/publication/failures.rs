use super::*;

#[test]
fn controlled_reviewed_source_closure_loss_and_tamper_preserve_valid_publication() -> TestResult {
    let (fixture, expected) = setup::prepare(Case::Resolved)?;
    let objects = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    let archive = tempfile::tempdir()?;
    let dataset = capture(&fixture, objects.path())?;
    witness(&dataset, &expected)?;
    let frozen = archive.path().join("controlled-authentic.json");
    dataset.save_frozen(&frozen)?;
    let wire = inspect_wire(&frozen)?;
    let root = output.path().join("publication");
    let workbook = publish(&fixture, &dataset, &root)?;
    let generation = workbook.parent().ok_or("controlled generation")?;
    let pointer = std::fs::read_link(root.join("current"))?;
    let digest = published::verify_published(&workbook)?.generation_digest;
    let rows = artifacts::readback(&workbook, &dataset, &expected)?;
    let manifest = std::fs::read(generation.join("manifest.json"))?;
    let workbook_bytes = std::fs::read(&workbook)?;
    let frozen_bytes = std::fs::read(generation.join("frozen-input.json"))?;
    let closure = wire
        .pointer("/selected_proofs/affiliation/archives/digest")
        .and_then(Value::as_str)
        .ok_or("controlled closure digest")?;
    let family = wire
        .pointer("/selected_proofs/affiliation/roster_family/digest")
        .and_then(Value::as_str)
        .ok_or("controlled family digest")?;
    let retained: Value = serde_json::from_slice(&std::fs::read(objects.path().join(closure))?)?;
    let reviewed = retained
        .pointer("/reviewed/digest")
        .and_then(Value::as_str)
        .ok_or("controlled reviewed bundle digest")?;
    for object in [closure, family, reviewed] {
        let path = objects.path().join(object);
        let original = std::fs::read(&path)?;
        check!(!original.is_empty());
        std::fs::remove_file(&path)?;
        check!(dataset
            .save_frozen(&archive.path().join("missing.json"))
            .is_err());
        check!(publish(&fixture, &dataset, &root).is_err());
        check!(eq; std::fs::read_link(root.join("current"))?, pointer);
        check!(eq; published::verify_published(&workbook)?.generation_digest, digest);
        std::fs::write(&path, &original)?;
        let mut tampered = original.clone();
        *tampered.first_mut().ok_or("controlled nonempty object")? ^= 1;
        std::fs::write(&path, tampered)?;
        check!(
            matches!(crate::export::proof_objects::read(objects.path(), object, 4 * 1024 * 1024), Err(crate::report::ReportError::ArtifactCorrupt { path: rejected, .. }) if rejected == path)
        );
        check!(dataset
            .save_frozen(&archive.path().join("tampered.json"))
            .is_err());
        check!(publish(&fixture, &dataset, &root).is_err());
        check!(eq; std::fs::read_link(root.join("current"))?, pointer);
        check!(eq; published::verify_published(&workbook)?.generation_digest, digest);
        std::fs::write(&path, original)?;
    }
    check!(eq; std::fs::read(generation.join("manifest.json"))?, manifest);
    check!(eq; std::fs::read(&workbook)?, workbook_bytes);
    check!(eq; std::fs::read(generation.join("frozen-input.json"))?, frozen_bytes);
    check!(eq; artifacts::readback(&workbook, &dataset, &expected)?, rows);
    check!(eq; publish(&fixture, &dataset, &root)?, workbook);
    check!(eq; published::verify_published(&workbook)?.generation_digest, digest);
    let reopened = ExportDataset::reopen_frozen_with_digest(&frozen, &dataset.archive_digest()?)?;
    witness(&reopened, &expected)?;
    Ok(())
}
