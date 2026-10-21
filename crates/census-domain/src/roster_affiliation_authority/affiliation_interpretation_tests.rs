use super::{
    decode_roster_issuance_claim, LocallyCheckedRosterIssuanceV1, RosterAuthorityLimitsV1,
    MAX_ROSTER_SEMANTIC_INTERPRETATION_BYTES,
};
use crate::affiliation_candidates::{
    AffiliationCandidateError, FinalUrl, LocallyCheckedEvidence, ObjectRef, RawEvidence, Resource,
};
use crate::model::{SourceCapture, SourceRef};
use sha2::{Digest, Sha256};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const BODY: &[u8] = b"SYNTHETIC ONLY: roster interpretation capacity boundary";

fn issuance(bytes: &[u8]) -> TestResult<LocallyCheckedRosterIssuanceV1> {
    let mut value = super::tests::fixture()?;
    *value
        .pointer_mut("/capture/interpretation")
        .ok_or("interpretation reference")? =
        serde_json::json!({"digest":format!("{:x}", Sha256::digest(bytes)),"bytes":bytes.len()});
    Ok(decode_roster_issuance_claim(
        &serde_json::to_vec(&value)?,
        RosterAuthorityLimitsV1::new(64 * 1024, 8 * 1024 * 1024)?,
    )?)
}

fn metadata(original: &LocallyCheckedRosterIssuanceV1) -> TestResult<SourceCapture> {
    Ok(SourceCapture::new(
        original.scope().owner(),
        format!("{:x}", Sha256::digest(BODY)),
        u64::try_from(BODY.len())?,
        original.capture().observed_day().as_str().into(),
        Some(original.capture().requested_url().into()),
    )?)
}

fn raw<'a>(
    original: &'a LocallyCheckedRosterIssuanceV1,
    bytes: &'a [u8],
    metadata: &'a SourceCapture,
    source: &'a SourceRef,
) -> TestResult<RawEvidence<'a>> {
    let capture = original.capture();
    let parser = original.parser();
    Ok(RawEvidence {
        occurrence: ObjectRef::parse(capture.occurrence().as_str())?,
        manifest_key: ObjectRef::parse(capture.manifest().digest().as_str())?,
        manifest_bytes: b"synthetic manifest",
        metadata,
        owner: original.scope().owner(),
        source,
        requested_url: capture.requested_url(),
        final_url: FinalUrl::Unknown,
        acquired_at: chrono::DateTime::parse_from_rfc3339(capture.acquired_at())?.to_utc(),
        body: BODY,
        interpretation_key: ObjectRef::parse(capture.interpretation().digest().as_str())?,
        interpretation_bytes: bytes,
        parser_artifact: ObjectRef::parse(parser.artifact().digest().as_str())?,
        extraction_schema: ObjectRef::parse(parser.schema().digest().as_str())?,
        extraction_config: ObjectRef::parse(parser.configuration().digest().as_str())?,
        reference_dependencies: ObjectRef::parse(parser.dependencies().digest().as_str())?,
    })
}

#[test]
fn roster_interpretation_bounds_do_not_expand_generic_metadata_admission() -> TestResult {
    for length in [
        64 * 1024,
        64 * 1024 + 1,
        MAX_ROSTER_SEMANTIC_INTERPRETATION_BYTES,
    ] {
        let bytes = vec![b'x'; length];
        let original = issuance(&bytes)?;
        let metadata = metadata(&original)?;
        let source = SourceRef::id("synthetic-roster-capacity");
        let raw = raw(&original, &bytes, &metadata, &source)?;
        let generic = LocallyCheckedEvidence::check(raw);
        if length == 64 * 1024 {
            check!(generic.is_ok());
        } else {
            check!(
                matches!(generic, Err(AffiliationCandidateError::BudgetExceeded {
                resource: Resource::MetadataBytes, needed, maximum: 65_536 }) if needed == length)
            );
        }
        check!(eq; LocallyCheckedEvidence::check_roster_interpretation(raw, &original)?.raw().interpretation_bytes, bytes.as_slice());
    }
    Ok(())
}

#[test]
fn roster_interpretation_requires_exact_checked_reference_key_length_and_digest() -> TestResult {
    let bytes = vec![b'x'; 64 * 1024 + 1];
    let original = issuance(&bytes)?;
    let metadata = metadata(&original)?;
    let source = SourceRef::id("synthetic-roster-capacity");
    let raw = raw(&original, &bytes, &metadata, &source)?;
    let mut changed = raw;
    changed.interpretation_key = ObjectRef::parse("different-original")?;
    check!(matches!(
        LocallyCheckedEvidence::check_roster_interpretation(changed, &original),
        Err(AffiliationCandidateError::InterpretationMismatch)
    ));
    changed = raw;
    changed.interpretation_bytes = bytes
        .get(..bytes.len().checked_sub(1).ok_or("interpretation length")?)
        .ok_or("short interpretation")?;
    check!(matches!(
        LocallyCheckedEvidence::check_roster_interpretation(changed, &original),
        Err(AffiliationCandidateError::InterpretationMismatch)
    ));
    let other = vec![b'y'; bytes.len()];
    changed = raw;
    changed.interpretation_bytes = &other;
    check!(matches!(
        LocallyCheckedEvidence::check_roster_interpretation(changed, &original),
        Err(AffiliationCandidateError::InterpretationMismatch)
    ));
    let different = issuance(&other)?;
    check!(matches!(
        LocallyCheckedEvidence::check_roster_interpretation(raw, &different),
        Err(AffiliationCandidateError::InterpretationMismatch)
    ));
    Ok(())
}

#[test]
fn roster_interpretation_admission_keeps_neighbor_limits_and_refuses_one_byte_over() -> TestResult {
    let bytes = vec![b'x'; MAX_ROSTER_SEMANTIC_INTERPRETATION_BYTES];
    let original = issuance(&bytes)?;
    let metadata = metadata(&original)?;
    let source = SourceRef::id("synthetic-roster-capacity");
    let raw = raw(&original, &bytes, &metadata, &source)?;
    let metadata_over = vec![b'm'; 65_537];
    let mut changed = raw;
    changed.manifest_bytes = &metadata_over;
    check!(matches!(
        LocallyCheckedEvidence::check_roster_interpretation(changed, &original),
        Err(AffiliationCandidateError::BudgetExceeded {
            resource: Resource::MetadataBytes,
            needed: 65_537,
            maximum: 65_536
        })
    ));
    let body_over = vec![b'b'; 4_194_305];
    changed = raw;
    changed.body = &body_over;
    check!(matches!(
        LocallyCheckedEvidence::check_roster_interpretation(changed, &original),
        Err(AffiliationCandidateError::BudgetExceeded {
            resource: Resource::BodyBytes,
            needed: 4_194_305,
            maximum: 4_194_304
        })
    ));
    let interpretation_over = vec![b'i'; MAX_ROSTER_SEMANTIC_INTERPRETATION_BYTES + 1];
    changed = raw;
    changed.interpretation_bytes = &interpretation_over;
    check!(
        matches!(LocallyCheckedEvidence::check_roster_interpretation(changed, &original), Err(AffiliationCandidateError::BudgetExceeded { resource: Resource::MetadataBytes, needed, maximum }) if needed == interpretation_over.len() && maximum == MAX_ROSTER_SEMANTIC_INTERPRETATION_BYTES)
    );
    Ok(())
}
