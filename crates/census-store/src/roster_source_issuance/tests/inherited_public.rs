use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::result_school_correspondence::{ResultSchoolReviewActorV1, ResultSchoolTextV1};
use census_store::{
    replay_frozen_roster_reviewed_claims_v1, Application, FrozenRosterReviewedClaimsBundleV1,
    ResultSchoolEnrollmentReviewV1, Store,
};
use chrono::DateTime;
use sha2::{Digest, Sha256};
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

pub(super) fn inherited_eight_retains_review_and_frozen_replay(
    store: Store,
    store_path: &Path,
    issue: impl Fn(&Store, &str) -> TestResult<Application>,
) -> TestResult {
    let original = issue(&store, "inherited-original-roster")
        .map_err(|error| format!("initial roster issuance: {error}"))?;
    check!(original.written());
    let (old, witness) =
        freeze(&store, 8, 1).map_err(|error| format!("predecessor frozen roster: {error}"))?;
    let digest = approval_digest(&store)?;
    enroll_school(&store)?;
    let (inherited, unchanged) =
        freeze(&store, 9, 1).map_err(|error| format!("inherited frozen roster: {error}"))?;
    check!(eq; unchanged, witness);
    check!(eq; store.replay_roster_source_review_v1(
        "inherited-original-roster", &digest)?.receipt(), original.receipt());
    replay(&old, 8, 1, &witness)?;
    replay(&inherited, 9, 1, &witness)?;
    reject_altered_proofs(&store, &inherited)?;
    let later = issue(&store, "inherited-later-roster")
        .map_err(|error| format!("later roster issuance: {error}"))?;
    check!(later.written());
    let (latest, preserved) =
        freeze(&store, 9, 2).map_err(|error| format!("later frozen roster: {error}"))?;
    check!(eq; preserved, witness);
    let identity = store.snapshot().store_identity()?;
    let sequence = store.snapshot().sequence();
    let input_token = store
        .inspect_result_school_correspondence_v1(AttachmentReplayGrantV1::new())?
        .input_token()
        .clone();
    drop(store);
    let (reopened, _writer) =
        Store::open_public_identity_qualification_v3(store_path)?.into_parts();
    check!(eq; reopened.snapshot().store_identity()?, identity);
    check!(eq; reopened.snapshot().sequence(), sequence);
    check!(eq; reopened
        .inspect_result_school_correspondence_v1(AttachmentReplayGrantV1::new())?
        .input_token(), &input_token);
    let (_, cold) = freeze(&reopened, 9, 2)?;
    check!(eq; cold, witness);
    check!(eq; reopened.replay_roster_source_review_v1(
        "inherited-original-roster", &digest)?.receipt(), original.receipt());
    replay(&old, 8, 1, &witness)?;
    replay(&latest, 9, 2, &witness)
}

fn enroll_school(store: &Store) -> TestResult {
    let inputs = store.inspect_result_school_enrollment_v1(AttachmentReplayGrantV1::new())?;
    let prepared = store.prepare_result_school_enrollment_v1(
        inputs,
        ResultSchoolEnrollmentReviewV1 {
            operation: ResultSchoolTextV1::parse("inherited-school-enrollment")?,
            actor: ResultSchoolReviewActorV1::LocalOperator,
            reviewed_at: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")?.to_utc(),
            rationale: ResultSchoolTextV1::parse(
                "Retain explicitly enrolled predecessor authority",
            )?,
        },
    )?;
    let bytes = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(store
        .apply_result_school_enrollment_v1(prepared, &bytes, &digest)?
        .written());
    check!(eq; store.store_format()?.lifecycle_revision, Some(9));
    Ok(())
}

fn freeze(
    store: &Store,
    lifecycle: u32,
    count: usize,
) -> TestResult<(FrozenRosterReviewedClaimsBundleV1, Vec<u8>)> {
    let mut grant = AttachmentReplayGrantV1::new();
    let current = store
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut grant)?;
    check!(eq; current.source_store_lifecycle(), lifecycle);
    let claims = current.claims(&mut grant)?;
    check!(eq; claims.len(), count);
    let first = claims.first().ok_or("nonempty reviewed roster required")?;
    check!(eq; first.operation(), "inherited-original-roster");
    let witness = serde_json::to_vec(first.proposal(&mut grant)?)?;
    let bundle = current.freeze_closure(&mut grant)?;
    require_wire_version(&bundle, lifecycle)?;
    Ok((bundle, witness))
}

fn replay(
    bundle: &FrozenRosterReviewedClaimsBundleV1,
    lifecycle: u32,
    count: usize,
    witness: &[u8],
) -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let replayed =
        replay_frozen_roster_reviewed_claims_v1(bundle.bytes(), bundle.reference(), &mut grant)?;
    check!(eq; replayed.source_store_lifecycle(), lifecycle);
    let claims = replayed.claims(&mut grant)?;
    check!(eq; claims.len(), count);
    let first = claims.first().ok_or("nonempty frozen roster required")?;
    check!(eq; first.operation(), "inherited-original-roster");
    check!(eq; serde_json::to_vec(first.proposal(&mut grant)?)?, witness);
    Ok(())
}

fn require_wire_version(bundle: &FrozenRosterReviewedClaimsBundleV1, lifecycle: u32) -> TestResult {
    let value: serde_json::Value = serde_json::from_slice(bundle.bytes())?;
    if lifecycle == 8 {
        check!(eq; value.get("schema"), Some(&serde_json::json!("roster-reviewed-source-publication-closure-v2")));
        check!(value.get("school_enrollment_proof").is_none());
        check!(eq; value.get("source_store_lifecycle"), Some(&serde_json::json!(8)));
    } else {
        check!(eq; value.get("schema"), Some(&serde_json::json!("roster-reviewed-source-publication-closure-v3")));
        check!(value.get("school_enrollment_proof").is_some());
        check!(eq; value.pointer("/closure/source_store_lifecycle"), Some(&serde_json::json!(9)));
    }
    Ok(())
}

fn approval_digest(store: &Store) -> TestResult<String> {
    let mut grant = AttachmentReplayGrantV1::new();
    let current = store
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut grant)?;
    let first = current
        .claims(&mut grant)?
        .first()
        .ok_or("review required")?;
    Ok(format!(
        "{:x}",
        Sha256::digest(first.approval_bytes(&mut grant)?)
    ))
}

fn identity_store() -> TestResult<(tempfile::TempDir, Store)> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("identity");
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
    Ok((directory, store))
}

pub(super) fn identity_only_nine_refuses_roster_authority() -> TestResult {
    let (_directory, store) = identity_store()?;
    check!(eq; store.store_format()?.lifecycle_revision, Some(7));
    enroll_school(&store)?;
    let sequence = store.snapshot().sequence();
    let result = store
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut AttachmentReplayGrantV1::new());
    check!(matches!(
        result,
        Err(census_store::StoreError::RosterSourceIssuance(
            census_store::RosterSourceIssuanceStoreError::EnrollmentRequired
        ))
    ));
    check!(matches!(
        store.inspect_roster_source_enrollment_v1(),
        Err(census_store::StoreError::RosterSourceIssuance(
            census_store::RosterSourceIssuanceStoreError::EnrollmentRequired
        ))
    ));
    check!(eq; store.snapshot().sequence(), sequence);
    Ok(())
}

fn reject_altered_proofs(store: &Store, bundle: &FrozenRosterReviewedClaimsBundleV1) -> TestResult {
    let sequence = store.snapshot().sequence();
    let raw = std::str::from_utf8(bundle.bytes())?;
    let marker = ",\"school_enrollment_proof\":";
    let start = raw.rfind(marker).ok_or("inherited proof field absent")?;
    let prefix = raw.get(..start).ok_or("proof prefix absent")?;
    let missing = format!("{prefix}}}").into_bytes();
    refuse(bundle, &missing)?;
    let (_directory, foreign) = identity_store()?;
    enroll_school(&foreign)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let proof = foreign
        .snapshot()
        .result_school_enrollment_proof_v1(&mut grant)?
        .ok_or("foreign proof absent")?;
    for replacement in [b"!".to_vec(), proof.bytes().to_vec()] {
        let encoded = serde_json::to_string(&replacement)?;
        let changed = format!("{prefix}{marker}{encoded}}}").into_bytes();
        refuse(bundle, &changed)?;
    }
    check!(eq; store.snapshot().sequence(), sequence);
    Ok(())
}

fn refuse(bundle: &FrozenRosterReviewedClaimsBundleV1, bytes: &[u8]) -> TestResult {
    let mut expected = serde_json::to_value(bundle.reference())?;
    *expected
        .get_mut("digest")
        .ok_or("reference digest absent")? =
        serde_json::json!(format!("{:x}", Sha256::digest(bytes)));
    *expected.get_mut("bytes").ok_or("reference bytes absent")? = serde_json::json!(bytes.len());
    let expected = serde_json::from_value(expected)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(replay_frozen_roster_reviewed_claims_v1(bytes, &expected, &mut grant).is_err());
    check!(grant.ensure_available().is_err());
    Ok(())
}
