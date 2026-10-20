use super::super::retention::files as retained;
use super::*;

const RETAIN_ENV: &str = "CENSUS_WITHDRAWAL_WORKBOOK_RETAIN_DIR";

pub(super) fn proof_refusal(
    fixture: &fixture::Fixture,
    dataset: &ExportDataset,
    frozen: &Path,
    objects_root: &Path,
    publication_root: &Path,
    workbook: &Path,
) -> TestResult {
    let wire = super::super::inspect_wire(frozen)?;
    let closure = wire
        .pointer("/selected_proofs/affiliation/archives/digest")
        .and_then(Value::as_str)
        .ok_or("controlled closure digest")?;
    let family = wire
        .pointer("/selected_proofs/affiliation/roster_family/digest")
        .and_then(Value::as_str)
        .ok_or("controlled family digest")?;
    let pointer = std::fs::read_link(publication_root.join("current"))?;
    let generation = crate::workbook::publication::verify_published(workbook)?.generation_digest;
    for digest in [closure, family] {
        let path = objects_root.join(digest);
        let original = std::fs::read(&path)?;
        std::fs::remove_file(&path)?;
        check!(dataset
            .save_frozen(
                &frozen
                    .parent()
                    .ok_or("frozen parent")?
                    .join("proof-refused.json")
            )
            .is_err());
        check!(super::super::publish(fixture, dataset, publication_root).is_err());
        check!(eq; std::fs::read_link(publication_root.join("current"))?, pointer);
        check!(eq; crate::workbook::publication::verify_published(workbook)?.generation_digest, generation);
        std::fs::write(&path, &original)?;
        let mut tampered = original.clone();
        *tampered.first_mut().ok_or("controlled proof")? ^= 1;
        std::fs::write(&path, &tampered)?;
        check!(matches!(
            crate::export::proof_objects::read(objects_root, digest, 4 * 1024 * 1024),
            Err(crate::report::ReportError::ArtifactCorrupt { path: rejected, .. }) if rejected == path
        ));
        check!(dataset
            .save_frozen(
                &frozen
                    .parent()
                    .ok_or("frozen parent")?
                    .join("proof-tampered.json")
            )
            .is_err());
        check!(super::super::publish(fixture, dataset, publication_root).is_err());
        check!(eq; std::fs::read_link(publication_root.join("current"))?, pointer);
        check!(eq; crate::workbook::publication::verify_published(workbook)?.generation_digest, generation);
        std::fs::write(&path, &original)?;
    }
    check!(eq; std::fs::read_link(publication_root.join("current"))?, pointer);
    check!(eq; crate::workbook::publication::verify_published(workbook)?.generation_digest, generation);
    Ok(())
}

pub(super) fn maybe_retain(
    frozen_g0: &Path,
    frozen_g1: &Path,
    workbook_g0: &Path,
    workbook_g1: &Path,
    digest_g0: &str,
    digest_g1: &str,
) -> TestResult {
    let Some(destination) = std::env::var_os(RETAIN_ENV) else {
        return Ok(());
    };
    let destination = PathBuf::from(destination);
    check!(destination.is_absolute());
    let parent = destination.parent().ok_or("retain parent")?;
    check!(eq; std::fs::canonicalize(parent)?, parent.to_path_buf());
    check!(std::fs::symlink_metadata(parent)?.is_dir());
    check!(
        matches!(std::fs::symlink_metadata(&destination), Err(error) if error.kind() == std::io::ErrorKind::NotFound)
    );
    let archive_dir = frozen_g0.parent().ok_or("retain archive")?;
    check!(eq; frozen_g1.parent(), Some(archive_dir));
    let generation_g0 = workbook_g0.parent().ok_or("retain generation")?;
    let generation_g1 = workbook_g1.parent().ok_or("retain generation")?;
    for source in [archive_dir, generation_g0, generation_g1] {
        let canonical = std::fs::canonicalize(source)?;
        check!(!destination.starts_with(&canonical));
        check!(!canonical.starts_with(&destination));
    }
    check!(!destination.starts_with(&std::env::current_dir()?));
    std::fs::create_dir(&destination)?;
    let archive_files = retained::copy(archive_dir, &destination.join("archive"))?;
    let generation_name_g0 = generation_g0
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("retain generation name")?;
    let retained_g0_root = destination.join("g0/publication/generations");
    std::fs::create_dir_all(&retained_g0_root)?;
    let g0_files = retained::copy(generation_g0, &retained_g0_root.join(generation_name_g0))?;
    let generation_name_g1 = generation_g1
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("retain generation name")?;
    let retained_g1_root = destination.join("g1/publication/generations");
    std::fs::create_dir_all(&retained_g1_root)?;
    let g1_files = retained::copy(generation_g1, &retained_g1_root.join(generation_name_g1))?;
    let frozen_name_g0 = frozen_g0
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("retain frozen name")?;
    let frozen_name_g1 = frozen_g1
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("retain frozen name")?;
    let workbook_name_g0 = workbook_g0
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("retain workbook name")?;
    let workbook_name_g1 = workbook_g1
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("retain workbook name")?;
    let manifest = json!({
        "frozen_g0": digest_g0,
        "frozen_g1": digest_g1,
        "archive_g0": format!("archive/{frozen_name_g0}"),
        "archive_g1": format!("archive/{frozen_name_g1}"),
        "workbook_g0": format!("g0/publication/generations/{generation_name_g0}/{workbook_name_g0}"),
        "workbook_g1": format!("g1/publication/generations/{generation_name_g1}/{workbook_name_g1}"),
        "archive_files": archive_files,
        "g0_files": g0_files,
        "g1_files": g1_files,
        "files": retained::inventory(&destination)?,
    });
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    let target = destination.join("withdrawal-retention.json");
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)?;
    use std::io::Write;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}
