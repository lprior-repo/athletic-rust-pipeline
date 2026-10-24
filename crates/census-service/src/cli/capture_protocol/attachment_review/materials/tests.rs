use super::super::tests::support::{sha256, Case, TestResult};
use super::*;
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentListV1, AttachmentObjectReferenceV1,
};
use census_store::AttachmentContextObjectV1;

#[test]
fn declared_inline_materials_borrow_the_exact_original_bytes_and_references() -> TestResult {
    let case = Case::new(false)?;
    let loaded = load(&case.command, &case.inline, &case.retained)?;
    let borrowed = loaded.borrowed()?;
    check!(eq; borrowed.len(), case.inline.len());
    for path in &case.inline {
        let original = std::fs::read(path)?;
        let digest = sha256(&original);
        let matching = borrowed
            .iter()
            .find_map(|object| match object {
                AttachmentContextObjectV1::Inline { reference, bytes }
                    if reference.digest.as_str() == digest =>
                {
                    Some((*reference, *bytes))
                }
                _ => None,
            })
            .ok_or("loaded inline reference absent")?;
        check!(eq; matching.0.bytes, u64::try_from(original.len())?);
        check!(eq; matching.1, original.as_slice());
    }
    Ok(())
}

#[test]
fn retained_leaf_supplier_preserves_exact_nonempty_root_object_and_original_length() -> TestResult {
    let case = Case::new(true)?;
    let leaf: census_store::RetainedCaptureLeafV1 = serde_json::from_slice(&std::fs::read(
        case.retained.first().ok_or("original leaf fixture")?,
    )?)?;
    check!(leaf.object.bytes > 0);
    let loaded = load(&case.command, &case.inline, &case.retained)?;
    let borrowed = loaded.borrowed()?;
    let found = borrowed
        .iter()
        .find_map(|object| match object {
            AttachmentContextObjectV1::Retained(found) => Some(*found),
            _ => None,
        })
        .ok_or("retained supplier lost")?;
    check!(eq; serde_json::to_vec(found)?, serde_json::to_vec(&leaf)?);
    check!(eq; borrowed.len(), case.inline.len() + 1);
    Ok(())
}

#[test]
fn duplicate_inline_and_retained_forms_of_the_same_object_refuse() -> TestResult {
    let mut case = Case::new(true)?;
    let body = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../census-crawl/src/participation_evidence/result_ownership/tests/fixtures/milesplit-result.html"));
    let path = case.directory.path().join("duplicate-original-body.bin");
    std::fs::write(&path, body)?;
    case.inline.push(path);
    let error = load(&case.command, &case.inline, &case.retained)
        .err()
        .ok_or("duplicate supplier forms accepted")?;
    check!(format!("{error:#}").contains("duplicate"));
    Ok(())
}

#[test]
fn malformed_and_oversized_leaf_json_refuse_at_the_preopen_material_boundary() -> TestResult {
    let case = Case::new(true)?;
    let path = case.retained.first().ok_or("leaf fixture")?;
    std::fs::write(path, b"{not-json")?;
    let error = load(&case.command, &case.inline, &case.retained)
        .err()
        .ok_or("malformed retained leaf accepted")?;
    check!(error.downcast_ref::<serde_json::Error>().is_some());
    std::fs::File::create(path)?.set_len(64 * 1024 + 1)?;
    let error = load(&case.command, &case.inline, &case.retained)
        .err()
        .ok_or("oversized retained leaf accepted")?;
    check!(eq; error.root_cause().to_string(), "attachment material exceeds its file limit");
    Ok(())
}

fn add_material(case: &mut Case, label: &str, bytes: &[u8]) -> TestResult<std::path::PathBuf> {
    let path = case.directory.path().join(label);
    std::fs::write(&path, bytes)?;
    let mut objects = case.command.reviewed_context.objects.as_slice().to_vec();
    objects.push(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&sha256(bytes))?,
        bytes: u64::try_from(bytes.len())?,
    });
    objects.sort();
    case.command.reviewed_context.objects = AttachmentListV1::new(objects)?;
    Ok(path)
}

#[test]
fn exactly_sixteen_mib_of_supplied_bytes_is_admitted_but_one_extra_byte_refuses() -> TestResult {
    let mut case = Case::new(false)?;
    let mut paths = Vec::new();
    let mut bytes = vec![0u8; 4 * 1024 * 1024];
    for marker in 1u8..=4 {
        *bytes.first_mut().ok_or("bounded buffer empty")? = marker;
        paths.push(add_material(
            &mut case,
            &format!("four-mib-{marker}.bin"),
            &bytes,
        )?);
    }
    drop(bytes);
    let loaded = load(&case.command, &paths, &[])?;
    let borrowed = loaded.borrowed()?;
    check!(eq; borrowed.len(), 4);
    let total = borrowed
        .iter()
        .try_fold(0u64, |total, object| -> TestResult<u64> {
            let AttachmentContextObjectV1::Inline { reference, bytes } = object else {
                return Err("unexpected retained material".into());
            };
            check!(eq; reference.bytes, u64::try_from(bytes.len())?);
            Ok(total
                .checked_add(reference.bytes)
                .ok_or("supplier bytes overflow")?)
        })?;
    check!(eq; total, 16 * 1024 * 1024);
    drop(borrowed);
    drop(loaded);
    paths.push(add_material(&mut case, "one-extra-byte.bin", b"x")?);
    let error = load(&case.command, &paths, &[])
        .err()
        .ok_or("16MiB plus one byte admitted")?;
    check!(eq; error.root_cause().to_string(), "attachment materials exceed the aggregate byte limit");
    Ok(())
}

#[test]
fn missing_material_path_and_supplier_count_fail_without_consulting_any_store() -> TestResult {
    let case = Case::new(false)?;
    let missing = case.directory.path().join("not-a-material-file");
    let error = load(&case.command, std::slice::from_ref(&missing), &[])
        .err()
        .ok_or("missing material accepted")?;
    check!(matches!(error.downcast_ref::<std::io::Error>(), Some(error)
        if error.kind() == std::io::ErrorKind::NotFound));
    let paths = vec![missing; 65];
    let error = load(&case.command, &paths, &[])
        .err()
        .ok_or("65 suppliers admitted")?;
    check!(format!("{error:#}").contains("64"));
    Ok(())
}
