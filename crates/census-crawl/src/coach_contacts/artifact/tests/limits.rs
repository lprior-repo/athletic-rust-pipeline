use super::super::{
    read_raw_contacts, read_verified_contacts, stage_verified_contacts, ContactArtifactError,
    Manifest, MAX_CSV_BYTES, MAX_MANIFEST_BYTES, MAX_RECORD_BYTES, MAX_ROWS,
};
use super::helpers::{sample_claims, sample_row, write_staged, TestResult};

#[test]
fn many_small_records_can_cross_the_per_record_limit_in_each_file() -> TestResult {
    let dir = tempfile::tempdir()?;
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
    )?;
    for file in ["contacts.csv", "contacts.csv.evidence.jsonl"] {
        check!(
            std::fs::metadata(staged.staging_dir.join(file))?.len()
                > u64::try_from(MAX_RECORD_BYTES)?
        );
    }
    let artifact = read_verified_contacts(&staged.staging_dir)?;
    check!(eq; artifact.row_count(), rows.len());
    for (actual, (expected, _)) in artifact.rows().iter().zip(&rows) {
        check!(eq; actual.row(), expected);
    }
    Ok(())
}

#[test]
fn an_oversized_csv_record_cannot_publish_a_readable_artifact() -> TestResult {
    let dir = tempfile::tempdir()?;
    let mut row = sample_row();
    row.city = "x".repeat(MAX_RECORD_BYTES);
    let claims = sample_claims(&row);
    let path = dir.path().join("stage");
    check!(matches!(
        stage_verified_contacts(&path, [(&row, claims.as_slice())]),
        Err(ContactArtifactError::Record { row: 1, .. })
    ));
    check!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::MissingFile { .. })
    ));
    let raw_path = dir.path().join("raw.csv");
    let raw = format!(
        "{}\n{},,,,,,,,,,\n",
        super::super::CONTACT_COLUMNS.join(","),
        "x".repeat(MAX_RECORD_BYTES)
    );
    std::fs::write(&raw_path, raw)?;
    check!(matches!(
        read_raw_contacts(&raw_path),
        Err(ContactArtifactError::Record { row: 1, .. })
    ));
    Ok(())
}

#[test]
fn oversized_files_and_manifest_counts_are_refused_before_unbounded_allocation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let raw = dir.path().join("oversized.csv");
    std::fs::File::create(&raw)?.set_len(MAX_CSV_BYTES + 1)?;
    check!(
        matches!(read_raw_contacts(&raw), Err(ContactArtifactError::CsvTooLarge { bytes, max, .. }) if bytes == MAX_CSV_BYTES + 1 && max == MAX_CSV_BYTES)
    );
    let row = sample_row();
    let path = write_staged(&dir, &row, &sample_claims(&row))?;
    let file = path.join("manifest.json");
    let mut manifest: Manifest = serde_json::from_slice(&std::fs::read(&file)?)?;
    manifest.verified_rows = usize::MAX;
    std::fs::write(&file, serde_json::to_vec(&manifest)?)?;
    check!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::RowLimitExceeded {
            count: usize::MAX,
            max: MAX_ROWS,
            ..
        })
    ));
    std::fs::write(file, vec![b' '; MAX_MANIFEST_BYTES + 1])?;
    check!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::ManifestParse { .. })
    ));
    Ok(())
}

#[test]
fn encoded_record_limit_includes_delimiters_and_line_endings() -> TestResult {
    let dir = tempfile::tempdir()?;
    for terminator in ["\n", "\r\n"] {
        let path = dir.path().join("boundary.csv");
        let value = "x".repeat(MAX_RECORD_BYTES - 10 - terminator.len());
        let record = format!("{value},,,,,,,,,,{terminator}");
        let raw = format!(
            "{}{terminator}{record}{record}",
            super::super::CONTACT_COLUMNS.join(",")
        );
        std::fs::write(&path, raw)?;
        let rows = read_raw_contacts(&path)?;
        check!(eq; rows.len(), 2);
        check!(rows.iter().all(|row| row.school == value));
        let oversized = format!(
            "{}\n{value}x,,,,,,,,,,{terminator}",
            super::super::CONTACT_COLUMNS.join(",")
        );
        std::fs::write(&path, oversized)?;
        check!(matches!(
            read_raw_contacts(&path),
            Err(ContactArtifactError::Record { row: 1, .. })
        ));
    }
    Ok(())
}

#[test]
fn oversized_evidence_envelopes_are_refused_on_write_and_read() -> TestResult {
    let dir = tempfile::tempdir()?;
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
    let proof = census_domain::model::compute_contact_proof(&row, &oversized)?;
    let path = dir.path().join("oversized");
    check!(matches!(
        stage_verified_contacts(&path, [(&row, oversized.as_slice())]),
        Err(ContactArtifactError::Record { path: file, row: 1, .. })
            if file == path.join(super::super::EVIDENCE_FILE)
    ));
    check!(matches!(
        read_verified_contacts(&path),
        Err(ContactArtifactError::MissingFile { .. })
    ));
    let valid = write_staged(&dir, &row, &claims)?;
    let envelope = super::super::EvidenceEnvelope {
        proof_digest: proof,
        claims: oversized,
    };
    let mut bytes = serde_json::to_vec(&envelope)?;
    bytes.push(b'\n');
    check!(bytes.len() > MAX_RECORD_BYTES);
    std::fs::write(valid.join(super::super::EVIDENCE_FILE), bytes)?;
    super::helpers::reseal(&valid, 1)?;
    check!(matches!(
        read_verified_contacts(&valid),
        Err(ContactArtifactError::JsonlParse { envelope: 1, .. })
    ));
    Ok(())
}
