use super::bounds::{check_limit, AffiliationCandidateError, ObjectRef, Resource};
use crate::model::{SourceCapture, SourceRef, SourceSubject};
use chrono::{DateTime, NaiveDate, Utc};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalUrl<'a> {
    Unknown,
    Observed(&'a str),
}

#[derive(Debug, Clone, Copy, Eq)]
pub struct RawEvidence<'a> {
    pub occurrence: ObjectRef<'a>,
    pub manifest_key: ObjectRef<'a>,
    pub manifest_bytes: &'a [u8],
    pub metadata: &'a SourceCapture,
    pub owner: &'a SourceSubject,
    pub source: &'a SourceRef,
    pub requested_url: &'a str,
    pub final_url: FinalUrl<'a>,
    pub acquired_at: DateTime<Utc>,
    pub body: &'a [u8],
    pub interpretation_key: ObjectRef<'a>,
    pub interpretation_bytes: &'a [u8],
    pub parser_artifact: ObjectRef<'a>,
    pub extraction_schema: ObjectRef<'a>,
    pub extraction_config: ObjectRef<'a>,
    pub reference_dependencies: ObjectRef<'a>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocallyCheckedEvidence<'a> {
    raw: RawEvidence<'a>,
}

impl<'a> LocallyCheckedEvidence<'a> {
    pub fn check(raw: RawEvidence<'a>) -> Result<Self, AffiliationCandidateError> {
        check_material(&raw)?;
        check_source(&raw)?;
        let date = NaiveDate::parse_from_str(raw.metadata.captured_at(), "%Y-%m-%d")
            .map_err(|_| AffiliationCandidateError::AcquisitionDateMismatch)?;
        if date != raw.acquired_at.date_naive() {
            return Err(AffiliationCandidateError::AcquisitionDateMismatch);
        }
        let length = u64::try_from(raw.body.len())
            .map_err(|_| AffiliationCandidateError::CounterOverflow)?;
        if raw.metadata.byte_len() != length {
            return Err(AffiliationCandidateError::LengthMismatch);
        }
        if !digest_matches(raw.body, raw.metadata.body_hash()) {
            return Err(AffiliationCandidateError::HashMismatch);
        }
        Ok(Self { raw })
    }

    pub const fn raw(&self) -> &RawEvidence<'a> {
        &self.raw
    }

    pub(crate) fn capture_byte_charge(&self) -> Result<usize, AffiliationCandidateError> {
        self.raw
            .body
            .len()
            .checked_add(self.raw.manifest_bytes.len())
            .ok_or(AffiliationCandidateError::CounterOverflow)
    }
}

fn check_material(raw: &RawEvidence<'_>) -> Result<(), AffiliationCandidateError> {
    if raw.body.is_empty() || raw.manifest_bytes.is_empty() {
        return Err(AffiliationCandidateError::MissingCaptureMaterial);
    }
    if raw.interpretation_bytes.is_empty() {
        return Err(AffiliationCandidateError::MissingInterpretationMaterial);
    }
    check_limit(Resource::BodyBytes, raw.body.len(), 4_194_304)?;
    check_limit(Resource::MetadataBytes, raw.manifest_bytes.len(), 65_536)?;
    check_limit(
        Resource::MetadataBytes,
        raw.interpretation_bytes.len(),
        65_536,
    )?;
    check_limit(
        Resource::MetadataBytes,
        raw.metadata.captured_at().len(),
        64,
    )
}

fn check_source(raw: &RawEvidence<'_>) -> Result<(), AffiliationCandidateError> {
    if raw.metadata.subject_id() != raw.owner.logical_id() {
        return Err(AffiliationCandidateError::ForeignCaptureOwner);
    }
    if raw.source.id.is_empty()
        || raw.source.id.len() > 512
        || raw.source.id.chars().any(char::is_control)
    {
        return Err(AffiliationCandidateError::InvalidCaptureSource);
    }
    check_url(raw.requested_url)?;
    if let Some(url) = &raw.source.url {
        check_url(url)?;
    }
    match raw.final_url {
        FinalUrl::Unknown => {}
        FinalUrl::Observed(url) => check_url(url)?,
    }
    if raw.metadata.source_url() != Some(raw.requested_url) {
        return Err(AffiliationCandidateError::InvalidCaptureSource);
    }
    Ok(())
}

fn check_url(value: &str) -> Result<(), AffiliationCandidateError> {
    if value.len() > 4096 || value.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(AffiliationCandidateError::InvalidCaptureSource);
    }
    let parsed =
        url::Url::parse(value).map_err(|_| AffiliationCandidateError::InvalidCaptureSource)?;
    if !matches!(parsed.scheme(), "http" | "https")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
    {
        return Err(AffiliationCandidateError::InvalidCaptureSource);
    }
    Ok(())
}

fn digest_matches(body: &[u8], expected: &str) -> bool {
    let actual = Sha256::digest(body);
    expected.len() == 64
        && expected
            .as_bytes()
            .chunks_exact(2)
            .zip(actual.iter())
            .all(|(pair, actual)| match pair {
                [high, low] => {
                    hex(*high)
                        .and_then(|high| high.checked_mul(16))
                        .and_then(|high| hex(*low).and_then(|low| high.checked_add(low)))
                        == Some(*actual)
                }
                _ => false,
            })
}

fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => byte.checked_sub(b'0'),
        b'a'..=b'f' => byte
            .checked_sub(b'a')
            .and_then(|digit| digit.checked_add(10)),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceSpan {
    from: usize,
    through_exclusive: usize,
}

impl SourceSpan {
    pub const MAX_BYTES: usize = 4096;

    pub fn range(self) -> std::ops::Range<usize> {
        self.from..self.through_exclusive
    }

    pub fn new(from: usize, through_exclusive: usize) -> Result<Self, AffiliationCandidateError> {
        let length = through_exclusive
            .checked_sub(from)
            .ok_or(AffiliationCandidateError::InvalidLocator)?;
        if length == 0 || length > Self::MAX_BYTES {
            return Err(AffiliationCandidateError::InvalidLocator);
        }
        Ok(Self {
            from,
            through_exclusive,
        })
    }

    pub fn read<'a>(
        self,
        evidence: &LocallyCheckedEvidence<'a>,
    ) -> Result<&'a [u8], AffiliationCandidateError> {
        evidence
            .raw
            .body
            .get(self.from..self.through_exclusive)
            .ok_or(AffiliationCandidateError::InvalidLocator)
    }
}
