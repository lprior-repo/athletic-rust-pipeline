use super::record::RecordBuffer;
use super::{
    BoundedHashWriter, ContactArtifactError, EvidenceEnvelope, Manifest, StagedContactArtifact,
    CONTACT_COLUMNS, CONTACT_PROOF_COLUMN, CSV_FILE, EVIDENCE_FILE, MANIFEST_FILE,
    MANIFEST_FORMAT_VERSION, MAX_CSV_BYTES, MAX_JSONL_BYTES, MAX_MANIFEST_BYTES, MAX_ROWS,
    VERIFIED_CONTACT_HEADER_COUNT,
};
use census_domain::model::{ContactClaimEvidence, RawContactRow};
use census_store::compute_contact_proof;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn stage_verified_contacts<'a>(
    fresh_staging_dir: &Path,
    rows: impl IntoIterator<Item = (&'a RawContactRow, &'a [ContactClaimEvidence])>,
) -> Result<StagedContactArtifact, ContactArtifactError> {
    std::fs::create_dir(fresh_staging_dir).map_err(|error| {
        if error.kind() == std::io::ErrorKind::AlreadyExists {
            ContactArtifactError::StagingPathExists {
                dir: fresh_staging_dir.to_owned(),
            }
        } else {
            ContactArtifactError::io(fresh_staging_dir, error)
        }
    })?;
    let mut writer = ArtifactWriter::new(fresh_staging_dir)?;
    for (row, claims) in rows {
        writer.append(row, claims)?;
    }
    writer.finish(fresh_staging_dir)
}

struct ArtifactWriter {
    csv: BoundedHashWriter<File>,
    evidence: BoundedHashWriter<File>,
    csv_path: PathBuf,
    evidence_path: PathBuf,
    record: RecordBuffer,
    urls: RecordBuffer,
    rows: usize,
}

impl ArtifactWriter {
    fn new(directory: &Path) -> Result<Self, ContactArtifactError> {
        let csv_path = directory.join(CSV_FILE);
        let evidence_path = directory.join(EVIDENCE_FILE);
        let csv = File::create_new(&csv_path)
            .map_err(|error| ContactArtifactError::io(&csv_path, error))?;
        let evidence = File::create_new(&evidence_path)
            .map_err(|error| ContactArtifactError::io(&evidence_path, error))?;
        let mut writer = Self {
            csv: BoundedHashWriter::new(csv, MAX_CSV_BYTES),
            evidence: BoundedHashWriter::new(evidence, MAX_JSONL_BYTES),
            record: RecordBuffer::new()
                .map_err(|error| ContactArtifactError::io(directory, error))?,
            urls: RecordBuffer::new()
                .map_err(|error| ContactArtifactError::io(directory, error))?,
            csv_path,
            evidence_path,
            rows: 0,
        };
        let mut header = [""; VERIFIED_CONTACT_HEADER_COUNT];
        for (cell, name) in header.iter_mut().zip(header_names()) {
            *cell = name;
        }
        writer.record.csv(header).map_err(|error| {
            ContactArtifactError::record(&writer.csv_path, 0, error.to_string())
        })?;
        writer.write_csv()?;
        Ok(writer)
    }

    fn append(
        &mut self,
        row: &RawContactRow,
        claims: &[ContactClaimEvidence],
    ) -> Result<(), ContactArtifactError> {
        if self.rows == MAX_ROWS {
            return Err(ContactArtifactError::RowLimitExceeded {
                path: self.csv_path.clone(),
                count: MAX_ROWS + 1,
                max: MAX_ROWS,
            });
        }
        let proof = compute_contact_proof(row, claims)?;
        self.write_row(row, &proof)?;
        self.write_envelope(claims, &proof)?;
        self.rows = self.rows.saturating_add(1);
        Ok(())
    }

    fn write_row(&mut self, row: &RawContactRow, proof: &str) -> Result<(), ContactArtifactError> {
        let row_number = self.rows.saturating_add(1);
        self.urls.json(&row.source_urls).map_err(|error| {
            ContactArtifactError::record(&self.csv_path, row_number, error.to_string())
        })?;
        let urls = self
            .urls
            .bytes()
            .map_err(|error| ContactArtifactError::io(&self.csv_path, error))?;
        let urls =
            std::str::from_utf8(urls).map_err(|error| ContactArtifactError::Serialization {
                detail: error.to_string(),
            })?;
        self.record
            .csv([
                &row.school,
                &row.city,
                &row.state,
                &row.sport,
                &row.role,
                &row.coach_name,
                &row.public_professional_email,
                &row.ad_name,
                &row.ad_email,
                urls,
                &row.last_observed,
                proof,
            ])
            .map_err(|error| {
                ContactArtifactError::record(&self.csv_path, row_number, error.to_string())
            })?;
        self.write_csv()
    }

    fn write_envelope(
        &mut self,
        claims: &[ContactClaimEvidence],
        proof: &str,
    ) -> Result<(), ContactArtifactError> {
        let row_number = self.rows.saturating_add(1);
        self.record
            .json(&EvidenceEnvelope {
                proof_digest: proof,
                claims,
            })
            .map_err(|error| {
                ContactArtifactError::record(&self.evidence_path, row_number, error.to_string())
            })?;
        self.record.write_all(b"\n").map_err(|error| {
            ContactArtifactError::record(&self.evidence_path, row_number, error.to_string())
        })?;
        let bytes = self
            .record
            .bytes()
            .map_err(|error| ContactArtifactError::io(&self.evidence_path, error))?;
        self.evidence
            .write_all(bytes)
            .map_err(|error| ContactArtifactError::io(&self.evidence_path, error))
    }

    fn write_csv(&mut self) -> Result<(), ContactArtifactError> {
        let bytes = self
            .record
            .bytes()
            .map_err(|error| ContactArtifactError::io(&self.csv_path, error))?;
        self.csv
            .write_all(bytes)
            .map_err(|error| ContactArtifactError::io(&self.csv_path, error))
    }

    fn finish(mut self, directory: &Path) -> Result<StagedContactArtifact, ContactArtifactError> {
        let manifest = Manifest {
            format_version: MANIFEST_FORMAT_VERSION,
            csv_sha256: sync_file(self.csv, &self.csv_path)?,
            evidence_sha256: sync_file(self.evidence, &self.evidence_path)?,
            verified_rows: self.rows,
        };
        let path = directory.join(MANIFEST_FILE);
        self.record.json(&manifest).map_err(serialization)?;
        self.record
            .write_all(b"\n")
            .map_err(|error| ContactArtifactError::io(&path, error))?;
        let bytes = self
            .record
            .bytes()
            .map_err(|error| ContactArtifactError::io(&path, error))?;
        if bytes.len() > MAX_MANIFEST_BYTES {
            return Err(ContactArtifactError::manifest_parse(
                &path,
                "manifest exceeds 4 KiB",
            ));
        }
        let mut file =
            File::create_new(&path).map_err(|error| ContactArtifactError::io(&path, error))?;
        file.write_all(bytes)
            .map_err(|error| ContactArtifactError::io(&path, error))?;
        file.sync_all()
            .map_err(|error| ContactArtifactError::io(&path, error))?;
        sync_directory(directory)?;
        let parent = match directory
            .parent()
            .filter(|path| !path.as_os_str().is_empty())
        {
            Some(value) => value,
            None => Path::new("."),
        };
        sync_directory(parent)?;
        Ok(StagedContactArtifact {
            staging_dir: directory.to_owned(),
            verified_rows: self.rows,
        })
    }
}

fn header_names() -> impl Iterator<Item = &'static str> {
    CONTACT_COLUMNS
        .iter()
        .copied()
        .chain(std::iter::once(CONTACT_PROOF_COLUMN))
}

fn sync_file(writer: BoundedHashWriter<File>, path: &Path) -> Result<String, ContactArtifactError> {
    let (file, digest) = writer.finish();
    file.sync_all()
        .map_err(|error| ContactArtifactError::io(path, error))?;
    Ok(digest)
}

fn sync_directory(path: &Path) -> Result<(), ContactArtifactError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| ContactArtifactError::io(path, error))
}

fn serialization(error: serde_json::Error) -> ContactArtifactError {
    ContactArtifactError::Serialization {
        detail: error.to_string(),
    }
}
