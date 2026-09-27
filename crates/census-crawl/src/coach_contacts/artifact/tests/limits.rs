use super::super::{
    read_raw_contacts, read_verified_contacts, stage_verified_contacts, ContactArtifactError,
    Manifest, MAX_CSV_BYTES, MAX_MANIFEST_BYTES, MAX_RECORD_BYTES, MAX_ROWS,
};
use super::helpers::{sample_claims, sample_row, write_staged};

#[test]
fn many_small_records_can_cross_the_per_record_limit_in_each_file() {
    let dir = tempfile::tempdir().unwrap();
    let padding = "x".repeat(1024);
    let rows: Vec<_> = (0..1024)
        .map(|index| {
            let mut row = sample_row();
            row.city = format!("City {index}: {padding}");
            let claims = sample_claims(&row);
            (row, claims)
        })
        .collect();
    let staged = stage_verified_contacts(
        &dir.path().join("stage"),
        rows.iter().map(|(row, claims)| (row, claims.as_slice())),
    )
    .unwrap();
    for file in ["contacts.csv", "contacts.csv.evidence.jsonl"] {
        assert!(
            std::fs::metadata(staged.staging_dir.join(file))
                .unwrap()
                .len()
                > u64::try_from(MAX_RECORD_BYTES).unwrap()
        );
    }
    let artifact = read_verified_contacts(&staged.staging_dir).unwrap();
    assert_eq!(artifact.row_count(), rows.len());
    for (actual, (expected, _)) in artifact.rows().iter().zip(&rows) {
        assert_eq!(actual.row(), expected);
    }
}

#[test]
fn an_oversized_csv_record_cannot_publish_a_readable_artifact() {
    let dir = tempfile::tempdir().unwrap();
    let mut row = sample_row();
    row.city = "x".repeat(MAX_RECORD_BYTES);
    let claims = sample_claims(&row);
    let path = dir.path().join("stage");
    assert!(matches!(
        stage_verified_contacts(&path, [(&row, claims.as_slice())]),
        Err(ContactArtifactError::Record { row: 1, .. })
    ));
    assert!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::MissingFile { .. })
    ));
    let raw_path = dir.path().join("raw.csv");
    let raw = format!(
        "{}\n{},,,,,,,,,,\n",
        super::super::CONTACT_COLUMNS.join(","),
        "x".repeat(MAX_RECORD_BYTES)
    );
    std::fs::write(&raw_path, raw).unwrap();
    assert!(matches!(
        read_raw_contacts(&raw_path),
        Err(ContactArtifactError::Record { row: 1, .. })
    ));
}

#[test]
fn oversized_files_and_manifest_counts_are_refused_before_unbounded_allocation() {
    let dir = tempfile::tempdir().unwrap();
    let raw = dir.path().join("oversized.csv");
    std::fs::File::create(&raw)
        .unwrap()
        .set_len(MAX_CSV_BYTES + 1)
        .unwrap();
    assert!(
        matches!(read_raw_contacts(&raw), Err(ContactArtifactError::CsvTooLarge { bytes, max, .. }) if bytes == MAX_CSV_BYTES + 1 && max == MAX_CSV_BYTES)
    );
    let row = sample_row();
    let path = write_staged(&dir, &row, &sample_claims(&row));
    let file = path.join("manifest.json");
    let mut manifest: Manifest = serde_json::from_slice(&std::fs::read(&file).unwrap()).unwrap();
    manifest.verified_rows = usize::MAX;
    std::fs::write(&file, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::RowLimitExceeded {
            count: usize::MAX,
            max: MAX_ROWS,
            ..
        })
    ));
    std::fs::write(file, vec![b' '; MAX_MANIFEST_BYTES + 1]).unwrap();
    assert!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::ManifestParse { .. })
    ));
}

#[test]
fn encoded_record_limit_includes_delimiters_and_line_endings() {
    let dir = tempfile::tempdir().unwrap();
    for terminator in ["\n", "\r\n"] {
        let path = dir.path().join("boundary.csv");
        let value = "x".repeat(MAX_RECORD_BYTES - 10 - terminator.len());
        let record = format!("{value},,,,,,,,,,{terminator}");
        let raw = format!(
            "{}{terminator}{record}{record}",
            super::super::CONTACT_COLUMNS.join(",")
        );
        std::fs::write(&path, raw).unwrap();
        let rows = read_raw_contacts(&path).unwrap();
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.school == value));
        let oversized = format!(
            "{}\n{value}x,,,,,,,,,,{terminator}",
            super::super::CONTACT_COLUMNS.join(",")
        );
        std::fs::write(&path, oversized).unwrap();
        assert!(matches!(
            read_raw_contacts(&path),
            Err(ContactArtifactError::Record { row: 1, .. })
        ));
    }
}

#[test]
fn oversized_evidence_envelopes_are_refused_on_write_and_read() {
    let dir = tempfile::tempdir().unwrap();
    let row = sample_row();
    let claims = sample_claims(&row);
    let oversized: Vec<_> = (0..64)
        .flat_map(|_| claims.iter().cloned())
        .map(|mut claim| {
            let padding = 4096 - claim.span.len();
            claim.span.push_str(&"x".repeat(padding));
            claim
        })
        .collect();
    let proof = census_domain::model::compute_contact_proof(&row, &oversized).unwrap();
    let path = dir.path().join("oversized");
    assert!(matches!(
        stage_verified_contacts(&path, [(&row, oversized.as_slice())]),
        Err(ContactArtifactError::Record { path: file, row: 1, .. })
            if file == path.join(super::super::EVIDENCE_FILE)
    ));
    assert!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::MissingFile { .. })
    ));
    let valid = write_staged(&dir, &row, &claims);
    let envelope = super::super::EvidenceEnvelope {
        proof_digest: proof,
        claims: oversized,
    };
    let mut bytes = serde_json::to_vec(&envelope).unwrap();
    bytes.push(b'\n');
    assert!(bytes.len() > MAX_RECORD_BYTES);
    std::fs::write(valid.join(super::super::EVIDENCE_FILE), bytes).unwrap();
    super::helpers::reseal(&valid, 1);
    assert!(matches!(
        read_verified_contacts(&valid),
        Err(ContactArtifactError::JsonlParse { envelope: 1, .. })
    ));
}
