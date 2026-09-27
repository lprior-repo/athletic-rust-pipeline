use super::super::tests::helpers::{reseal, sample_claims, sample_row, write_staged};
use super::super::{
    read_verified_contacts, stage_verified_contacts, ContactArtifactError, EvidenceEnvelope,
    Manifest,
};

#[test]
fn extra_jsonl_record_is_not_ignored() {
    let dir = tempfile::tempdir().unwrap();
    let row = sample_row();
    let path = write_staged(&dir, &row, &sample_claims(&row));
    let extra: EvidenceEnvelope = EvidenceEnvelope {
        proof_digest: "b".repeat(64),
        claims: vec![],
    };
    let file = path.join("contacts.csv.evidence.jsonl");
    let mut bytes = std::fs::read_to_string(&file).unwrap();
    bytes.push_str(&serde_json::to_string(&extra).unwrap());
    bytes.push('\n');
    std::fs::write(file, bytes).unwrap();
    reseal(&path, 1);
    assert!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::EnvelopeCountMismatch {
            csv_rows: 1,
            jsonl_envelopes: 2,
            ..
        })
    ));
}

#[test]
fn missing_envelopes_report_the_actual_csv_population() {
    let dir = tempfile::tempdir().unwrap();
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
    )
    .unwrap()
    .staging_dir;
    std::fs::write(path.join("contacts.csv.evidence.jsonl"), "").unwrap();
    reseal(&path, 2);
    assert!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::EnvelopeCountMismatch {
            csv_rows: 2,
            jsonl_envelopes: 0,
            ..
        })
    ));
}

#[test]
fn an_extra_csv_row_cannot_borrow_another_rows_evidence() {
    let dir = tempfile::tempdir().unwrap();
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
    )
    .unwrap()
    .staging_dir;
    let evidence_path = path.join("contacts.csv.evidence.jsonl");
    let evidence = std::fs::read_to_string(&evidence_path).unwrap();
    std::fs::write(
        evidence_path,
        evidence.lines().next().unwrap().to_owned() + "\n",
    )
    .unwrap();
    reseal(&path, 2);
    assert!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::EnvelopeCountMismatch {
            csv_rows: 2,
            jsonl_envelopes: 1,
            ..
        })
    ));
}

#[test]
fn changed_file_bytes_fail_manifest_integrity_before_semantic_acceptance() {
    for file in ["contacts.csv", "contacts.csv.evidence.jsonl"] {
        let dir = tempfile::tempdir().unwrap();
        let row = sample_row();
        let path = write_staged(&dir, &row, &sample_claims(&row));
        let content = std::fs::read_to_string(path.join(file))
            .unwrap()
            .replace("Jane Doe", "Changed Person");
        std::fs::write(path.join(file), content).unwrap();
        assert!(matches!(
            read_verified_contacts(&path),
            Err(ContactArtifactError::ManifestHash { .. })
        ));
    }
}

#[test]
fn changing_csv_proof_and_rehashing_does_not_change_the_evidence_proof() {
    let dir = tempfile::tempdir().unwrap();
    let row = sample_row();
    let path = write_staged(&dir, &row, &sample_claims(&row));
    let envelope: EvidenceEnvelope =
        serde_json::from_slice(&std::fs::read(path.join("contacts.csv.evidence.jsonl")).unwrap())
            .unwrap();
    let csv_path = path.join("contacts.csv");
    let csv = std::fs::read_to_string(&csv_path)
        .unwrap()
        .replace(&envelope.proof_digest, &"a".repeat(64));
    std::fs::write(csv_path, csv).unwrap();
    reseal(&path, 1);
    assert!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::ProofValidation { row: 1, .. })
    ));
}

#[test]
fn changing_evidence_and_rehashing_does_not_verify_the_original_row() {
    let dir = tempfile::tempdir().unwrap();
    let row = sample_row();
    let path = write_staged(&dir, &row, &sample_claims(&row));
    let evidence_path = path.join("contacts.csv.evidence.jsonl");
    let mut envelope: EvidenceEnvelope =
        serde_json::from_slice(&std::fs::read(&evidence_path).unwrap()).unwrap();
    envelope.claims[0].value = "Changed Name".to_owned();
    std::fs::write(evidence_path, serde_json::to_vec(&envelope).unwrap()).unwrap();
    reseal(&path, 1);
    assert!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::ProofValidation { row: 1, .. })
    ));
}

#[test]
fn incorrect_manifest_version_row_count_and_json_are_rejected() {
    for case in 0..3 {
        let dir = tempfile::tempdir().unwrap();
        let row = sample_row();
        let path = write_staged(&dir, &row, &sample_claims(&row));
        let file = path.join("manifest.json");
        let mut manifest: Manifest =
            serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
        match case {
            0 => manifest.format_version = 99,
            1 => manifest.verified_rows = 999,
            _ => {}
        }
        std::fs::write(&file, serde_json::to_vec(&manifest).unwrap()).unwrap();
        if case == 2 {
            std::fs::write(file, "{invalid json").unwrap();
        }
        match (case, read_verified_contacts(&path).unwrap_err()) {
            (0, ContactArtifactError::ManifestField { .. })
            | (
                1,
                ContactArtifactError::ManifestRowCountMismatch {
                    manifest: 999,
                    actual: 1,
                    ..
                },
            )
            | (2, ContactArtifactError::ManifestParse { .. }) => {}
            (_, error) => panic!("unexpected manifest failure: {error:?}"),
        }
    }
}
