use super::{Fixture, IdentityMap, TestResult};
use census_domain::public_identity_history::local_adjudication::{
    ClaimTreatmentV1, LocalIdentitySplitV1, RetainedClaimDispositionV1, SupersededMergeV1,
};
use census_domain::public_identity_history::{BindingPartition, HistoryRequest, PublicAthleteId};
use census_store::Table;

pub(super) fn split(
    fixture: &Fixture,
    case_id: &str,
    canonical: &PublicAthleteId,
) -> TestResult<IdentityMap> {
    let operation = "controlled-pr-local-split";
    let head = fixture.writer.head(&fixture.store)?;
    let inventory =
        fixture
            .writer
            .inspect_local_adjudication(&fixture.store, operation, canonical)?;
    for performance in [&fixture.eligible, &fixture.stronger] {
        check!(inventory
            .affected_record_keys()
            .contains(&format!("performances:{}", performance.id)));
    }
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
            rationale: "Controlled correction rejects only synthetic alias corroboration".into(),
        })
        .collect();
    let accepted = fixture
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("merged history")?;
    let mut partitions = accepted
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
    partitions.sort_by(|left, right| left.binding_ids.cmp(&right.binding_ids));
    check!(eq; partitions.len(), 2);
    let representatives: Vec<_> = partitions
        .iter()
        .map(|row| row.representative.to_string())
        .collect();
    let request = LocalIdentitySplitV1 {
        case_id: case_id.into(), canonical: canonical.clone(),
        supersedes: vec![SupersededMergeV1 { sequence: head.sequence, digest: head.digest.clone() }],
        partitions, unresolved_bindings: Vec::new(),
        affected_record_keys: inventory.affected_record_keys().to_vec(), dispositions,
        rationale: "Controlled eligible PR split preserves retained source performances and prior publication".into(),
    };
    let physical = fixture.store.snapshot().tables_digest(&Table::ALL)?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        operation,
        request,
        fixture.store.snapshot().revision(),
        &head,
    )?;
    let bytes = prepared.approval_bytes()?;
    let digest = prepared.approval_digest()?;
    let outcome =
        fixture
            .writer
            .apply_local_adjudication(&fixture.store, prepared, bytes, digest)?;
    check!(eq; outcome.introduced_public_ids().len(), 2);
    check!(eq; fixture.store.snapshot().tables_digest(&Table::ALL)?, physical);
    Ok(representatives
        .into_iter()
        .zip(outcome.introduced_public_ids().iter().cloned())
        .collect())
}
