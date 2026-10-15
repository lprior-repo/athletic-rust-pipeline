use super::{published_aliases, selected_dataset, Derivation, Scope};
use crate::export::public_identity::tests::fixture::{
    prepare_merge, source_births, Fixture, TestResult,
};
use census_domain::public_identity_history::local_adjudication::{
    ClaimTreatmentV1, LocalIdentitySplitV1, RetainedClaimDispositionV1, SupersededMergeV1,
};
use census_domain::public_identity_history::{BindingPartition, HistoryRequest};
use census_store::PreparedLocalAdjudicationV1;

#[test]
fn committed_local_split_restores_distinct_candidate_joins_and_keeps_merged_snapshot_stable(
) -> TestResult {
    let fixture = source_births()?;
    let reviewed = prepare_merge(&fixture)?;
    let census_store::ReviewedIdentityRequestV1::Merge { case_id, .. } = &reviewed else {
        return Err("expected reviewed merge fixture".into());
    };
    fixture.writer.apply_reviewed_identity(
        &fixture.store,
        "consumer-before-local-split",
        &reviewed,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let before = selected_dataset(&fixture.store)?;
    let merged = Derivation::for_publication(&before, Scope::AllSources, None, &[])?;
    check!(eq; merged.athletes().len(), 1);
    let before_rows = published_aliases(fixture.directory.path(), &merged)?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "consumer-committed-local-split",
        request(&fixture, case_id)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let (bytes, digest) = approval(&prepared)?;
    let outcome =
        fixture
            .writer
            .apply_local_adjudication(&fixture.store, prepared, bytes, digest)?;
    check!(eq; outcome.introduced_public_ids().len(), 2);
    let after = selected_dataset(&fixture.store)?;
    let split = Derivation::for_publication(&after, Scope::AllSources, None, &[])?;
    check!(eq; split.athletes().len(), 2);
    let rows = published_aliases(fixture.directory.path(), &split)?;
    let view = after.verified_public_identities()?;
    let [first, second, unknown] = &fixture.athletes;
    check!(ne; view.public_id(first.id.as_str()), view.public_id(second.id.as_str()));
    for candidate in [first, second] {
        let public_id = view
            .public_id(candidate.id.as_str())
            .ok_or("split public ID")?;
        check!(outcome.introduced_public_ids().contains(public_id));
        let row = rows.get(candidate.id.as_str()).ok_or("split alias")?;
        check!(eq; row.get("canonical_id").map(String::as_str), Some(candidate.id.as_str()));
        check!(eq; row.get("resolved_id").map(String::as_str), Some(candidate.id.as_str()));
        check!(eq; row.get("public_athlete_id").map(String::as_str), Some(public_id.as_str()));
        check!(eq; row.get("publication_membership").map(String::as_str), Some("canonical_in_publication"));
    }
    let withheld = rows.get(unknown.id.as_str()).ok_or("unknown alias")?;
    check!(eq; withheld.get("public_athlete_id").map(String::as_str), Some(""));
    check!(eq; withheld.get("publication_membership").map(String::as_str), Some("canonical_outside_publication"));
    check!(eq; before.athletes, after.athletes);
    check!(eq; before.canonical_aliases, after.canonical_aliases);
    check!(eq; published_aliases(fixture.directory.path(), &merged)?, before_rows);
    Ok(())
}
fn request(fixture: &Fixture, case_id: &str) -> TestResult<LocalIdentitySplitV1> {
    let canonical = fixture.births.first().ok_or("first birth")?.public_id();
    let head = fixture.writer.head(&fixture.store)?;
    let inventory = fixture.writer.inspect_local_adjudication(
        &fixture.store,
        "consumer-committed-local-split",
        canonical,
    )?;
    let dispositions = inventory.claims().iter().map(|claim| RetainedClaimDispositionV1 {
        claim: claim.clone(),
        treatment: if claim.claim_path.starts_with("/source_links/")
            || claim.claim_path.starts_with("/identity_attestations/") {
            ClaimTreatmentV1::RejectIdentityCorroboration
        } else { ClaimTreatmentV1::RetainAsContext },
        rationale: "Synthetic selected-proof review rejects the link between separately retained subjects".into(),
    }).collect();
    let accepted = fixture
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("accepted identity")?;
    let partitions = accepted
        .history_records()
        .iter()
        .filter_map(|record| {
            if let HistoryRequest::Establish { binding, .. } = &record.request {
                Some(BindingPartition {
                    representative: binding.candidate.clone(),
                    binding_ids: vec![binding.id.clone()],
                })
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    check!(eq; partitions.len(), 2);
    Ok(LocalIdentitySplitV1 {
        case_id: case_id.into(), canonical: canonical.clone(),
        supersedes: vec![SupersededMergeV1 { sequence: head.sequence, digest: head.digest }],
        partitions, unresolved_bindings: Vec::new(),
        affected_record_keys: inventory.affected_record_keys().to_vec(),
        dispositions,
        rationale: "Synthetic explicit local correction reverses the accepted identity interpretation without rewriting retained source evidence".into(),
    })
}

fn approval(prepared: &PreparedLocalAdjudicationV1<'_>) -> TestResult<(Vec<u8>, String)> {
    Ok((prepared.approval_bytes()?, prepared.approval_digest()?))
}
