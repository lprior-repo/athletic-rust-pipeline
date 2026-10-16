use super::{require_population, selected_dataset, selected_report, Scanned};
use crate::export::public_identity::tests::fixture::{
    prepare_merge, source_births, Fixture, TestResult,
};
use census_domain::public_identity_history::local_adjudication::{
    ClaimTreatmentV1, LocalIdentitySplitV1, RetainedClaimDispositionV1, SupersededMergeV1,
};
use census_domain::public_identity_history::{BindingPartition, HistoryRequest};
use census_domain::UsJurisdiction;
use census_store::PreparedLocalAdjudicationV1;
use serde::Serialize;
use sha2::{Digest, Sha256};

#[test]
fn local_split_reconciles_two_accepted_people_and_preserves_the_merged_snapshot() -> TestResult {
    let fixture = source_births()?;
    let reviewed = prepare_merge(&fixture)?;
    let census_store::ReviewedIdentityRequestV1::Merge { case_id, .. } = &reviewed else {
        return Err("expected reviewed merge fixture".into());
    };
    fixture.writer.apply_reviewed_identity(
        &fixture.store,
        "coverage-before-local-split",
        &reviewed,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let before = selected_dataset(&fixture.store)?;
    let merged = selected_report(&before, &[])?;
    require_population(&merged, 1, 1, 0)?;
    let before_bytes = serde_json::to_vec(&before)?;
    split(&fixture, case_id)?;
    let after = selected_dataset(&fixture.store)?;
    check!(eq; before.athletes, after.athletes);
    check!(eq; before.canonical_aliases, after.canonical_aliases);
    let scanned = Scanned::of(&after)?;
    check!(eq; scanned.athletes.len(), 2);
    check!(scanned.aliases.is_empty());
    require_population(&selected_report(&after, &[])?, 2, 2, 0)?;
    require_population(
        &selected_report(&after, &[UsJurisdiction::Wisconsin])?,
        2,
        0,
        2,
    )?;
    require_population(&selected_report(&before, &[])?, 1, 1, 0)?;
    check!(eq; serde_json::to_vec(&before)?, before_bytes);
    Ok(())
}

pub(crate) fn split(fixture: &Fixture, case_id: &str) -> TestResult {
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "coverage-committed-local-split",
        request(fixture, case_id)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let (bytes, digest) = approval(&prepared)?;
    fixture
        .writer
        .apply_local_adjudication(&fixture.store, prepared, bytes, digest)?;
    Ok(())
}

fn request(fixture: &Fixture, case_id: &str) -> TestResult<LocalIdentitySplitV1> {
    let canonical = fixture.births.first().ok_or("first birth")?.public_id();
    let head = fixture.writer.head(&fixture.store)?;
    let inventory = fixture
        .writer
        .inspect_local_adjudication(&fixture.store, canonical)?;
    let dispositions = inventory
        .claims()
        .iter()
        .map(|claim| RetainedClaimDispositionV1 {
            claim: claim.clone(),
            treatment: if claim.claim_path.starts_with("/source_links/")
                || claim.claim_path.starts_with("/identity_attestations/")
            {
                ClaimTreatmentV1::RejectIdentityCorroboration
            } else {
                ClaimTreatmentV1::RetainAsContext
            },
            rationale:
                "Synthetic coverage correction rejects the link between separate retained subjects"
                    .into(),
        })
        .collect();
    let accepted = fixture
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("accepted identity")?;
    let partitions = accepted
        .history_records()
        .iter()
        .filter_map(|record| match &record.request {
            HistoryRequest::Establish { binding, .. } => Some(BindingPartition {
                representative: binding.candidate.clone(),
                binding_ids: vec![binding.id.clone()],
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    check!(eq; partitions.len(), 2);
    Ok(LocalIdentitySplitV1 {
        case_id: case_id.into(),
        canonical: canonical.clone(),
        supersedes: vec![SupersededMergeV1 {
            sequence: head.sequence,
            digest: head.digest,
        }],
        partitions,
        unresolved_bindings: Vec::new(),
        affected_record_keys: inventory.affected_record_keys().to_vec(),
        dispositions,
        rationale: "Synthetic explicit coverage correction preserves all retained source evidence"
            .into(),
    })
}

fn approval(prepared: &PreparedLocalAdjudicationV1<'_>) -> TestResult<(Vec<u8>, String)> {
    #[derive(Serialize)]
    struct Envelope<'a, T: Serialize> {
        schema: &'static str,
        actor: &'static str,
        prepared: &'a T,
    }
    let bytes = serde_json::to_vec(&Envelope {
        schema: "local-identity-adjudication-approval/v1",
        actor: "local_operator",
        prepared: prepared.record(),
    })?;
    let mut digest = Sha256::new();
    digest.update(b"local-identity-adjudication-approval/v1");
    digest.update([0]);
    digest.update(&bytes);
    Ok((bytes, format!("{:x}", digest.finalize())))
}
