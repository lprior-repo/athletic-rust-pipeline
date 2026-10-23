use super::*;
use census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError;
use census_domain::{model::CanonicalSchool, UsJurisdiction};

fn append_school(store: &Store) -> TestResult {
    let school = CanonicalSchool::new(
        UsJurisdiction::Michigan,
        format!("Controlled manifest {}", "x".repeat(4096)),
        "controlled-manifest-school",
        None,
    )
    .0;
    store.append_many(crate::Table::Schools, &[school])?;
    Ok(())
}

fn require_byte_capacity(error: crate::StoreError, needed: usize) -> TestResult {
    check!(matches!(
        error,
        crate::StoreError::EvidenceSubjectAttachment(
            crate::EvidenceSubjectAttachmentStoreError::Domain(
                EvidenceSubjectAttachmentError::Capacity {
                    resource: "aggregate encoded bytes",
                    needed: actual,
                    available: AttachmentReplayGrantV1::MAX_ENCODED_BYTES,
                }
            )
        ) if actual == needed
    ));
    Ok(())
}

#[test]
fn borrowed_row_hashes_charge_work_and_each_manifest_output_once() -> TestResult {
    let (_directory, store) = enrolled()?;
    append_school(&store)?;
    let mut inspection = store
        .inspect_roster_source_issuance_v1(request(&store)?, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let rows = evidence.physical_projection(grant)?.len();
    check!(rows > 0);
    let before = grant.usage();
    let first = evidence.physical_projection_manifest(grant)?;
    check!(eq;
        grant.usage().encoded_bytes().checked_sub(before.encoded_bytes()),
        Some(first.len())
    );
    check!(eq;
        grant.usage().traversal_steps().checked_sub(before.traversal_steps()),
        rows.checked_mul(3).and_then(|work| work.checked_add(5))
    );
    let padding = AttachmentReplayGrantV1::MAX_ENCODED_BYTES
        .checked_sub(grant.usage().encoded_bytes())
        .and_then(|remaining| remaining.checked_sub(first.len()))
        .ok_or("manifest grant padding overflow")?;
    grant.charge_bytes(padding)?;
    check!(eq; evidence.physical_projection_manifest(grant)?, first);
    check!(eq; grant.usage().encoded_bytes(), AttachmentReplayGrantV1::MAX_ENCODED_BYTES);
    let error = evidence
        .physical_projection_manifest(grant)
        .err()
        .ok_or("manifest output exceeded the exact byte grant")?;
    require_byte_capacity(
        error,
        AttachmentReplayGrantV1::MAX_ENCODED_BYTES
            .checked_add(first.len())
            .ok_or("manifest refusal size overflow")?,
    )?;
    check!(eq; grant.usage().encoded_bytes(), AttachmentReplayGrantV1::MAX_ENCODED_BYTES);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn one_byte_short_manifest_output_poisons_the_original_grant() -> TestResult {
    let (_directory, store) = enrolled()?;
    append_school(&store)?;
    let selected = request(&store)?;
    let root = selected.root.clone();
    let school = selected.school.clone();
    let program = selected.program.clone();
    let mut reference =
        store.inspect_roster_source_issuance_v1(selected, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = reference.source_parts()?;
    let expected = evidence.physical_projection_manifest(grant)?;
    let mut inspection = store.inspect_roster_source_issuance_v1(
        RosterIssuanceInspectionRequestV1 {
            root,
            school,
            program,
        },
        AttachmentReplayGrantV1::new(),
    )?;
    let (evidence, grant) = inspection.source_parts()?;
    let padding = AttachmentReplayGrantV1::MAX_ENCODED_BYTES
        .checked_sub(grant.usage().encoded_bytes())
        .and_then(|remaining| remaining.checked_sub(expected.len()))
        .and_then(|remaining| remaining.checked_add(1))
        .ok_or("manifest short grant padding overflow")?;
    grant.charge_bytes(padding)?;
    let before = grant.usage().encoded_bytes();
    let error = evidence
        .physical_projection_manifest(grant)
        .err()
        .ok_or("one-byte-short grant admitted manifest output")?;
    require_byte_capacity(
        error,
        AttachmentReplayGrantV1::MAX_ENCODED_BYTES
            .checked_add(1)
            .ok_or("manifest short refusal overflow")?,
    )?;
    check!(eq; grant.usage().encoded_bytes(), before);
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    check!(evidence.physical_projection_manifest(grant).is_err());
    Ok(())
}
