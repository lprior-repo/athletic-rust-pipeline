use super::csv_reader::ContactCsv;
use super::envelopes::Envelopes;
use super::{
    ContactArtifactError, EvidenceEnvelope, Manifest, ValidatedRow, VerifiedContactArtifact,
    CONTACT_COLUMNS, CONTACT_PROOF_COLUMN, CSV_FILE, EVIDENCE_FILE, MANIFEST_FILE,
    MANIFEST_FORMAT_VERSION, MAX_MANIFEST_BYTES, MAX_ROWS, RAW_CONTACT_HEADER_COUNT,
    VERIFIED_CONTACT_HEADER_COUNT,
};
use census_domain::model::{verify_contact_proof, RawContactRow};
use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

pub fn read_raw_contacts(path: &Path) -> Result<Vec<RawContactRow>, ContactArtifactError> {
    let mut csv = ContactCsv::open(path, RAW_CONTACT_HEADER_COUNT)?;
    header(&mut csv, path, RAW_CONTACT_HEADER_COUNT)?;
    let mut rows = Vec::new();
    for row in 1..=MAX_ROWS.saturating_add(1) {
        let Some(record) = csv.next(row)? else {
            return Ok(rows);
        };
        enforce_rows(path, row)?;
        let source_urls = field(&record, path, row, 9)?
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        rows.try_reserve(1)
            .map_err(|error| ContactArtifactError::io(path, io::Error::other(error)))?;
        rows.push(raw_row(&record, source_urls, path, row)?);
    }
    Err(ContactArtifactError::RowLimitExceeded {
        path: path.to_owned(),
        count: MAX_ROWS + 1,
        max: MAX_ROWS,
    })
}

pub fn read_verified_contacts(
    directory: &Path,
) -> Result<VerifiedContactArtifact, ContactArtifactError> {
    let manifest = read_manifest(directory)?;
    let csv_path = directory.join(CSV_FILE);
    let evidence_path = directory.join(EVIDENCE_FILE);
    let mut csv = ContactCsv::open(&csv_path, VERIFIED_CONTACT_HEADER_COUNT)?;
    header(&mut csv, &csv_path, VERIFIED_CONTACT_HEADER_COUNT)?;
    let mut envelopes = Envelopes::open(&evidence_path)?;
    let rows = read_pairs(
        &mut csv,
        &mut envelopes,
        &manifest,
        &csv_path,
        &evidence_path,
    );
    let csv_digest = csv.digest()?;
    let evidence_digest = envelopes.digest()?;
    check_digest(&csv_path, "csv", &manifest.csv_sha256, &csv_digest)?;
    check_digest(
        &evidence_path,
        "jsonl",
        &manifest.evidence_sha256,
        &evidence_digest,
    )?;
    Ok(VerifiedContactArtifact {
        rows: rows?,
        directory: directory.to_owned(),
    })
}

fn read_pairs(
    csv: &mut ContactCsv,
    envelopes: &mut Envelopes,
    manifest: &Manifest,
    csv_path: &Path,
    evidence_path: &Path,
) -> Result<Vec<ValidatedRow>, ContactArtifactError> {
    let mut rows = Vec::new();
    let (mut csv_rows, mut jsonl_envelopes) = (0usize, 0usize);
    for index in 1..=MAX_ROWS.saturating_add(1) {
        let record = csv.next(index)?;
        let envelope = envelopes.next()?;
        if record.is_none() && envelope.is_none() {
            if csv_rows != jsonl_envelopes {
                return Err(ContactArtifactError::EnvelopeCountMismatch {
                    path: evidence_path.to_owned(),
                    csv_rows,
                    jsonl_envelopes,
                });
            }
            if csv_rows != manifest.verified_rows {
                return Err(ContactArtifactError::ManifestRowCountMismatch {
                    path: csv_path.to_owned(),
                    manifest: manifest.verified_rows,
                    actual: csv_rows,
                });
            }
            return Ok(rows);
        }
        if record.is_some() {
            csv_rows = csv_rows.saturating_add(1);
            enforce_rows(csv_path, csv_rows)?;
        }
        if envelope.is_some() {
            jsonl_envelopes = jsonl_envelopes.saturating_add(1);
            enforce_rows(evidence_path, jsonl_envelopes)?;
        }
        if let (Some(record), Some(envelope)) = (record, envelope) {
            rows.try_reserve(1)
                .map_err(|error| ContactArtifactError::io(csv_path, io::Error::other(error)))?;
            rows.push(validate_row(&record, envelope, csv_path, csv_rows)?);
        }
    }
    Err(ContactArtifactError::RowLimitExceeded {
        path: csv_path.to_owned(),
        count: MAX_ROWS + 1,
        max: MAX_ROWS,
    })
}

fn header(csv: &mut ContactCsv, path: &Path, columns: usize) -> Result<(), ContactArtifactError> {
    let record = csv
        .next(0)?
        .ok_or_else(|| ContactArtifactError::HeaderMismatch {
            path: path.to_owned(),
            expected: columns,
            actual: 0,
            detail: "missing header".to_owned(),
        })?;
    for (index, expected) in CONTACT_COLUMNS
        .iter()
        .copied()
        .chain(std::iter::once(CONTACT_PROOF_COLUMN))
        .take(columns)
        .enumerate()
    {
        if record.get(index).copied() != Some(expected) {
            return Err(ContactArtifactError::HeaderMismatch {
                path: path.to_owned(),
                expected: columns,
                actual: columns,
                detail: format!("column {index}: expected `{expected}`"),
            });
        }
    }
    Ok(())
}

fn validate_row(
    record: &[&str; VERIFIED_CONTACT_HEADER_COUNT],
    envelope: EvidenceEnvelope,
    path: &Path,
    index: usize,
) -> Result<ValidatedRow, ContactArtifactError> {
    let source_urls: Vec<String> =
        serde_json::from_str(field(record, path, index, 9)?).map_err(|error| {
            ContactArtifactError::InvalidSourceUrlsJson {
                path: path.to_owned(),
                row: index,
                detail: error.to_string(),
            }
        })?;
    if source_urls.iter().any(String::is_empty) {
        return Err(ContactArtifactError::EmptySourceUrl { row: index });
    }
    let csv_proof = field(record, path, index, 11)?;
    if csv_proof.is_empty() {
        return Err(ContactArtifactError::MissingProofDigest { row: index });
    }
    if !is_hex64(csv_proof) {
        return Err(ContactArtifactError::MalformedProofDigest {
            row: index,
            detail: "expected 64 lowercase hexadecimal characters".to_owned(),
        });
    }
    if csv_proof != envelope.proof_digest {
        return Err(ContactArtifactError::proof(
            index,
            "CSV and evidence proof differ",
        ));
    }
    let row = raw_row(record, source_urls, path, index)?;
    let proof = verify_contact_proof(&row, &envelope.claims, csv_proof)
        .map_err(|error| ContactArtifactError::proof(index, error.to_string()))?;
    Ok(ValidatedRow {
        row,
        claims: envelope.claims,
        proof,
    })
}

fn raw_row(
    record: &[&str; VERIFIED_CONTACT_HEADER_COUNT],
    source_urls: Vec<String>,
    path: &Path,
    index: usize,
) -> Result<RawContactRow, ContactArtifactError> {
    Ok(RawContactRow {
        school: field(record, path, index, 0)?.to_owned(),
        city: field(record, path, index, 1)?.to_owned(),
        state: field(record, path, index, 2)?.to_owned(),
        sport: field(record, path, index, 3)?.to_owned(),
        role: field(record, path, index, 4)?.to_owned(),
        coach_name: field(record, path, index, 5)?.to_owned(),
        public_professional_email: field(record, path, index, 6)?.to_owned(),
        ad_name: field(record, path, index, 7)?.to_owned(),
        ad_email: field(record, path, index, 8)?.to_owned(),
        source_urls,
        last_observed: field(record, path, index, 10)?.to_owned(),
    })
}

fn field<'a>(
    record: &'a [&'a str; VERIFIED_CONTACT_HEADER_COUNT],
    path: &Path,
    row: usize,
    position: usize,
) -> Result<&'a str, ContactArtifactError> {
    record
        .get(position)
        .copied()
        .ok_or_else(|| ContactArtifactError::record(path, row, format!("missing field {position}")))
}

fn read_manifest(directory: &Path) -> Result<Manifest, ContactArtifactError> {
    let path = directory.join(MANIFEST_FILE);
    let file = File::open(&path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            ContactArtifactError::MissingFile {
                dir: directory.to_owned(),
                file: MANIFEST_FILE.to_owned(),
            }
        } else {
            ContactArtifactError::io(&path, error)
        }
    })?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(MAX_MANIFEST_BYTES + 1)
        .map_err(|error| ContactArtifactError::io(&path, io::Error::other(error)))?;
    let limit = u64::try_from(MAX_MANIFEST_BYTES + 1)
        .map_err(|error| ContactArtifactError::io(&path, io::Error::other(error)))?;
    file.take(limit)
        .read_to_end(&mut bytes)
        .map_err(|error| ContactArtifactError::io(&path, error))?;
    if bytes.len() > MAX_MANIFEST_BYTES {
        return Err(ContactArtifactError::manifest_parse(
            &path,
            "manifest exceeds 4 KiB",
        ));
    }
    let manifest: Manifest = serde_json::from_slice(&bytes)
        .map_err(|error| ContactArtifactError::manifest_parse(&path, error.to_string()))?;
    if manifest.format_version != MANIFEST_FORMAT_VERSION {
        return Err(ContactArtifactError::manifest_field(
            &path,
            format!(
                "expected version {MANIFEST_FORMAT_VERSION}, found {}",
                manifest.format_version
            ),
        ));
    }
    enforce_rows(&path, manifest.verified_rows)?;
    if !is_hex64(&manifest.csv_sha256) || !is_hex64(&manifest.evidence_sha256) {
        return Err(ContactArtifactError::manifest_field(
            &path,
            "manifest hashes must be 64 lowercase hexadecimal characters",
        ));
    }
    Ok(manifest)
}

fn enforce_rows(path: &Path, count: usize) -> Result<(), ContactArtifactError> {
    if count > MAX_ROWS {
        return Err(ContactArtifactError::RowLimitExceeded {
            path: path.to_owned(),
            count,
            max: MAX_ROWS,
        });
    }
    Ok(())
}

fn check_digest(
    path: &Path,
    kind: &str,
    expected: &str,
    actual: &str,
) -> Result<(), ContactArtifactError> {
    if expected != actual {
        return Err(ContactArtifactError::ManifestHash {
            path: path.to_owned(),
            detail: format!("{kind} sha256 mismatch: manifest={expected}, actual={actual}"),
        });
    }
    Ok(())
}

fn is_hex64(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
