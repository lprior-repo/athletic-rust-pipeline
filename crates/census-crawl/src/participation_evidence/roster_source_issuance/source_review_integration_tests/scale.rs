use super::*;
use census_domain::model::{SourceNamespace, SourceObservation, SourceSchoolObservation};
use census_store::{
    replay_frozen_roster_reviewed_claims_v1, FrozenRosterReviewedClaimsBundleV1,
    ReviewedRosterSourceClaimV1, Table,
};
use sha2::{Digest, Sha256};

#[path = "scale_tamper.rs"]
mod tamper;

const OPERATION: &str = "controlled-physical-context-above-former-ceilings";
const CONTEXT_ROWS: usize = 4097;

struct Witness {
    rows: usize,
    manifest: Vec<u8>,
    manifest_digest: String,
    last_key: Vec<u8>,
    last_raw: Vec<u8>,
}

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn context_id(index: usize) -> String {
    format!("{}-{index:05}", "x".repeat(320))
}

fn install_context(store: &Store) -> TestResult {
    for start in (0..CONTEXT_ROWS).step_by(32) {
        let rows: Vec<_> = (start..(start + 32).min(CONTEXT_ROWS))
            .map(|index| {
                let mut row = SourceSchoolObservation::new(
                    SourceNamespace::MilesplitSchool,
                    format!("{}", 9_000_000 + index),
                    format!("controlled-context-{index}"),
                    "Unrelated controlled context",
                    "2026-10-16",
                );
                row.id = context_id(index);
                SourceObservation::School(row)
            })
            .collect();
        store.append_many(Table::SourceObservations, &rows)?;
    }
    Ok(())
}

fn witness(prepared: &mut PreparedRosterSourceReviewV1<'_>) -> TestResult<Witness> {
    let claim = prepared.proposal_claim()?;
    let (evidence, grant) = prepared.source_parts()?;
    let manifest = evidence.physical_projection_manifest(grant)?;
    check!(manifest.len() > 4 * 1024 * 1024);
    check!(eq; claim.binding().physical_projection_inventory().bytes().get(), u64::try_from(manifest.len())?);
    let manifest_digest = hash(&manifest);
    check!(eq; claim.binding().physical_projection_inventory().digest().as_str(), manifest_digest.as_str());
    let rows = evidence.physical_projection(grant)?;
    check!(eq; rows.len(), CONTEXT_ROWS + 3);
    let last_id = context_id(CONTEXT_ROWS - 1);
    let last = rows
        .iter()
        .find(|row| {
            row.table() == Table::SourceObservations
                && row
                    .key()
                    .windows(last_id.len())
                    .any(|part| part == last_id.as_bytes())
        })
        .ok_or("last controlled physical context row absent")?;
    let observation: SourceObservation = serde_json::from_slice(last.bytes())?;
    check!(eq; observation.id(), last_id.as_str());
    Ok(Witness {
        rows: rows.len(),
        manifest,
        manifest_digest,
        last_key: last.key().to_vec(),
        last_raw: last.bytes().to_vec(),
    })
}

fn require_witness(
    claim: &ReviewedRosterSourceClaimV1,
    expected: &Witness,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult {
    check!(eq; claim.operation(), OPERATION);
    check!(eq; claim.object(&expected.manifest_digest, grant)?, expected.manifest.as_slice());
    check!(eq; claim.object(&hash(&expected.last_raw), grant)?, expected.last_raw.as_slice());
    let rows = claim.evidence(grant)?.physical_projection(grant)?;
    check!(eq; rows.len(), expected.rows);
    let last = rows
        .iter()
        .find(|row| row.key() == expected.last_key.as_slice())
        .ok_or("last retained physical context row absent")?;
    check!(eq; last.table(), Table::SourceObservations);
    check!(eq; last.bytes(), expected.last_raw.as_slice());
    Ok(())
}

fn diagnostics(store: &Store) -> TestResult<(u64, String, u64)> {
    let snapshot = store.snapshot();
    Ok((
        snapshot.sequence(),
        snapshot.tables_digest(&Table::ALL)?,
        store.receipt_count()?,
    ))
}

#[test]
fn real_large_physical_manifest_preserves_exact_retry_and_cold_frozen_replay() -> TestResult {
    let (directory, store) = enrolled_store()?;
    install_context(&store)?;
    let mut prepared = prepare(&store, OPERATION, None)?;
    let expected = witness(&mut prepared)?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let applied = store.apply_roster_source_review_v1(prepared, &approval, &digest)?;
    check!(applied.written());
    check!(eq; applied.receipt().appended, 0);
    check!(!applied.receipt().credited);
    let after = diagnostics(&store)?;
    let repeated = store.replay_roster_source_review_v1(OPERATION, &digest)?;
    check!(repeated.repeated());
    check!(eq; repeated.receipt(), applied.receipt());
    check!(eq; diagnostics(&store)?, after);
    check!(store
        .replay_roster_source_review_v1(OPERATION, &"d".repeat(64))
        .is_err());
    check!(eq; diagnostics(&store)?, after);
    drop(store);
    let (reopened, _writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("controlled-roster"))?
            .into_parts();
    check!(eq; diagnostics(&reopened)?, after);
    let cold = reopened.replay_roster_source_review_v1(OPERATION, &digest)?;
    check!(cold.repeated());
    check!(eq; cold.receipt(), applied.receipt());
    check!(eq; diagnostics(&reopened)?, after);
    let mut grant = AttachmentReplayGrantV1::new();
    let current = reopened
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut grant)?;
    let claims = current.claims(&mut grant)?;
    check!(eq; claims.len(), 1);
    require_witness(
        claims.first().ok_or("review absent")?,
        &expected,
        &mut grant,
    )?;
    let bundle = current.freeze_closure(&mut grant)?;
    drop(reopened);
    require_frozen_witness(&bundle, &expected)?;
    tamper::refuse_changed_or_missing_last_object(&bundle, &expected)?;
    require_frozen_witness(&bundle, &expected)
}

fn require_frozen_witness(
    bundle: &FrozenRosterReviewedClaimsBundleV1,
    expected: &Witness,
) -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let replayed =
        replay_frozen_roster_reviewed_claims_v1(bundle.bytes(), bundle.reference(), &mut grant)?;
    let claims = replayed.claims(&mut grant)?;
    check!(eq; claims.len(), 1);
    require_witness(
        claims.first().ok_or("frozen review absent")?,
        expected,
        &mut grant,
    )
}

#[test]
fn large_physical_context_refuses_changed_approval_and_stale_basis_without_writes() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    install_context(&store)?;
    let prepared = prepare(&store, "large-context-changed-approval", None)?;
    let digest = prepared.approval_digest().to_owned();
    let before = diagnostics(&store)?;
    check!(matches!(
        store.apply_roster_source_review_v1(prepared, b"{}", &digest),
        Err(census_store::StoreError::RosterSourceIssuance(
            census_store::RosterSourceIssuanceStoreError::ApprovalMismatch
        ))
    ));
    check!(eq; diagnostics(&store)?, before);
    check!(store
        .replay_roster_source_review_v1("large-context-changed-approval", &digest)
        .is_err());
    check!(eq; diagnostics(&store)?, before);
    let prepared = prepare(&store, "large-context-stale-basis", None)?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    install(&store)?;
    let before = diagnostics(&store)?;
    check!(store
        .apply_roster_source_review_v1(prepared, &approval, &digest)
        .is_err());
    check!(eq; diagnostics(&store)?, before);
    check!(store
        .replay_roster_source_review_v1("large-context-stale-basis", &digest)
        .is_err());
    check!(eq; diagnostics(&store)?, before);
    Ok(())
}
