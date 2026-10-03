use super::super::tests::helpers::{reseal, sample_claims, sample_row, write_staged, TestResult};
use super::super::{
    read_verified_contacts, stage_verified_contacts, ContactArtifactError, EvidenceEnvelope,
    Manifest,
};

#[test]
fn extra_jsonl_record_is_not_ignored() -> TestResult {
    let dir = tempfile::tempdir()?;
    let row = sample_row();
    let path = write_staged(&dir, &row, &sample_claims(&row))?;
    let extra: EvidenceEnvelope = EvidenceEnvelope {
        proof_digest: "b".repeat(64),
        claims: vec![],
    };
    let file = path.join("contacts.csv.evidence.jsonl");
    let mut bytes = std::fs::read_to_string(&file)?;
    bytes.push_str(&serde_json::to_string(&extra)?);
    bytes.push('\n');
    std::fs::write(file, bytes)?;
    reseal(&path, 1)?;
    check!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::EnvelopeCountMismatch {
            csv_rows: 1,
            jsonl_envelopes: 2,
            ..
        })
    ));
    Ok(())
}

#[test]
fn missing_envelopes_report_the_actual_csv_population() -> TestResult {
    let dir = tempfile::tempdir()?;
    let first = sample_row();
    let mut second = sample_row();
    second.school = "Second High".to_owned();
    let first_claims = sample_claims(&first);
    let second_claims = sample_claims(&second);
    let path = stage_verified_contacts(
        &dir.path().join("stage"),
        [
            (&first, first_claims.as_slice()),
            (&second, second_claims.as_slice()),
        ],
    )?
    .staging_dir;
    std::fs::write(path.join("contacts.csv.evidence.jsonl"), "")?;
    reseal(&path, 2)?;
    check!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::EnvelopeCountMismatch {
            csv_rows: 2,
            jsonl_envelopes: 0,
            ..
        })
    ));
    Ok(())
}

#[test]
fn an_extra_csv_row_cannot_borrow_another_rows_evidence() -> TestResult {
    let dir = tempfile::tempdir()?;
    let first = sample_row();
    let mut second = sample_row();
    second.school = "Second High".to_owned();
    let first_claims = sample_claims(&first);
    let second_claims = sample_claims(&second);
    let path = stage_verified_contacts(
        &dir.path().join("stage"),
        [
            (&first, first_claims.as_slice()),
            (&second, second_claims.as_slice()),
        ],
    )?
    .staging_dir;
    let evidence_path = path.join("contacts.csv.evidence.jsonl");
    let evidence = std::fs::read_to_string(&evidence_path)?;
    std::fs::write(
        evidence_path,
        evidence
            .lines()
            .next()
            .ok_or("first evidence envelope")?
            .to_owned()
            + "\n",
    )?;
    reseal(&path, 2)?;
    check!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::EnvelopeCountMismatch {
            csv_rows: 2,
            jsonl_envelopes: 1,
            ..
        })
    ));
    Ok(())
}

#[test]
fn changed_file_bytes_fail_manifest_integrity_before_semantic_acceptance() -> TestResult {
    for file in ["contacts.csv", "contacts.csv.evidence.jsonl"] {
        let dir = tempfile::tempdir()?;
        let row = sample_row();
        let path = write_staged(&dir, &row, &sample_claims(&row))?;
        let content =
            std::fs::read_to_string(path.join(file))?.replace("Jane Doe", "Changed Person");
        std::fs::write(path.join(file), content)?;
        check!(matches!(
            read_verified_contacts(&path),
            Err(ContactArtifactError::ManifestHash { .. })
        ));
    }
    Ok(())
}

#[test]
fn changing_csv_proof_and_rehashing_does_not_change_the_evidence_proof() -> TestResult {
    let dir = tempfile::tempdir()?;
    let row = sample_row();
    let path = write_staged(&dir, &row, &sample_claims(&row))?;
    let envelope: EvidenceEnvelope =
        serde_json::from_slice(&std::fs::read(path.join("contacts.csv.evidence.jsonl"))?)?;
    let csv_path = path.join("contacts.csv");
    let csv = std::fs::read_to_string(&csv_path)?.replace(&envelope.proof_digest, &"a".repeat(64));
    std::fs::write(csv_path, csv)?;
    reseal(&path, 1)?;
    check!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::ProofValidation { row: 1, .. })
    ));
    Ok(())
}

#[test]
fn changing_evidence_and_rehashing_does_not_verify_the_original_row() -> TestResult {
    let dir = tempfile::tempdir()?;
    let row = sample_row();
    let path = write_staged(&dir, &row, &sample_claims(&row))?;
    let evidence_path = path.join("contacts.csv.evidence.jsonl");
    let mut envelope: EvidenceEnvelope = serde_json::from_slice(&std::fs::read(&evidence_path)?)?;
    envelope.claims[0].value = "Changed Name".to_owned();
    std::fs::write(evidence_path, serde_json::to_vec(&envelope)?)?;
    reseal(&path, 1)?;
    check!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::ProofValidation { row: 1, .. })
    ));
    Ok(())
}

#[test]
fn incorrect_manifest_version_row_count_and_json_are_rejected() -> TestResult {
    for case in 0..3 {
        let dir = tempfile::tempdir()?;
        let row = sample_row();
        let path = write_staged(&dir, &row, &sample_claims(&row))?;
        let file = path.join("manifest.json");
        let mut manifest: Manifest = serde_json::from_slice(&std::fs::read(&file)?)?;
        match case {
            0 => manifest.format_version = 99,
            1 => manifest.verified_rows = 999,
            _ => {}
        }
        std::fs::write(&file, serde_json::to_vec(&manifest)?)?;
        if case == 2 {
            std::fs::write(file, "{invalid json")?;
        }
        match (case, read_verified_contacts(&path)) {
            (0, Err(ContactArtifactError::ManifestField { .. }))
            | (
                1,
                Err(ContactArtifactError::ManifestRowCountMismatch {
                    manifest: 999,
                    actual: 1,
                    ..
                }),
            )
            | (2, Err(ContactArtifactError::ManifestParse { .. })) => {}
            (_, outcome) => return Err(format!("unexpected manifest outcome: {outcome:?}").into()),
        }
    }
    Ok(())
}
