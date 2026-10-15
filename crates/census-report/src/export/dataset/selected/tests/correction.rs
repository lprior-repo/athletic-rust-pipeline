use super::{fixture, witness, ExportDataset, Table, TestResult};
use crate::export::public_identity::tests::fixture::{prepare_merge, source_births, Fixture};
use census_domain::model::ProfileSelectionV1;
use census_domain::public_identity_history::local_adjudication::{
    ClaimTreatmentV1, LocalIdentitySplitV1, RetainedClaimDispositionV1, SupersededMergeV1,
};
use census_domain::public_identity_history::{BindingPartition, HistoryRequest, PublicResolution};
use census_store::{PreparedLocalAdjudicationV1, ReviewedIdentityRequestV1};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[test]
fn journal_only_local_identity_correction_invalidates_current_selection_and_preserves_prior_archive(
) -> TestResult {
    let fixture = source_births()?;
    let merge = prepare_merge(&fixture)?;
    fixture.writer.apply_reviewed_identity(
        &fixture.store,
        "selected-proof-prior-merge",
        &merge,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let profile = fixture::profile()?;
    fixture.store.register_profile(&profile)?;
    let selection = ProfileSelectionV1::new(fixture.store.store_identity()?, profile.id()?);
    let objects = super::temporary()?;
    let held = fixture.store.snapshot();
    let dataset =
        ExportDataset::from_selected_snapshot(&fixture.store, &held, &selection, objects.path())?;
    let archive = super::temporary()?;
    let original_path = archive.path().join("before-correction.json");
    dataset.save_frozen(&original_path)?;
    let original_digest = dataset.archive_digest()?;
    let original = witness::capture(&dataset)?;
    let [first, second, unknown] = &fixture.athletes;
    let before = dataset.verified_public_identities()?;
    check!(eq; before.group(first.id.as_str()).ok_or("merged group")?.members().count(), 2);
    check!(eq; before.public_id(first.id.as_str()), before.public_id(second.id.as_str()));
    let physical = held.tables_digest(&Table::ALL)?;
    fixture
        .store
        .journal_done("selected-proof-unrelated", "progress", &true)?;
    dataset.ensure_current(&fixture.store)?;
    let ReviewedIdentityRequestV1::Merge { case_id, .. } = &merge else {
        return Err("expected reviewed merge request".into());
    };
    let correction = request(&fixture, case_id)?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "selected-proof-local-split",
        correction,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let (bytes, digest) = approval(&prepared)?;
    let outcome =
        fixture
            .writer
            .apply_local_adjudication(&fixture.store, prepared, bytes, digest)?;
    check!(eq; outcome.introduced_public_ids().len(), 2);
    check!(eq; fixture.store.snapshot().tables_digest(&Table::ALL)?, physical);
    check!(eq; crate::export::store_lineage(&fixture.store)?.source_digest, dataset.lineage.source_digest);
    check!(dataset.ensure_current(&fixture.store).is_err());
    check!(dataset.ensure_snapshot(&fixture.store.snapshot()).is_err());
    dataset.ensure_snapshot(&held)?;
    let fresh_objects = super::temporary()?;
    let refreshed = ExportDataset::from_selected_snapshot(
        &fixture.store,
        &fixture.store.snapshot(),
        &selection,
        fresh_objects.path(),
    )?;
    check!(eq; refreshed.lineage.source_digest, dataset.lineage.source_digest);
    check!(ne; refreshed.lineage.input_generation, dataset.lineage.input_generation);
    check!(eq; refreshed.verified_public_identities()?.head(), outcome.head());
    let after = refreshed.verified_public_identities()?;
    check!(ne; after.public_id(first.id.as_str()), after.public_id(second.id.as_str()));
    for candidate in [first, second] {
        let group = after.group(candidate.id.as_str()).ok_or("split group")?;
        check!(eq; group.canonical_candidate().as_str(), candidate.id.as_str());
        check!(eq; group.members().count(), 1);
        check!(group.public_id().is_some());
    }
    check!(eq; after.public_id(unknown.id.as_str()), None);
    check!(eq; refreshed.athletes, dataset.athletes);
    check!(eq; refreshed.identity_decisions, dataset.identity_decisions);
    check!(eq; refreshed.canonical_aliases, dataset.canonical_aliases);
    let canonical = fixture.births.first().ok_or("original birth")?.public_id();
    check!(matches!(
        refreshed
            .verified_public_identities()?
            .resolve_public(canonical)?,
        PublicResolution::Ambiguous { .. }
    ));
    let current_path = archive.path().join("after-correction.json");
    refreshed.save_frozen(&current_path)?;
    let current_digest = refreshed.archive_digest()?;
    let current = witness::capture(&refreshed)?;
    check!(ne; current_digest, original_digest);
    drop(held);
    drop(dataset);
    drop(refreshed);
    let Fixture {
        directory,
        store,
        writer,
        ..
    } = fixture;
    drop(writer);
    drop(store);
    directory.close()?;
    objects.close()?;
    fresh_objects.close()?;
    check!(eq; witness::capture(&ExportDataset::reopen_frozen_with_digest(&original_path, &original_digest)?)?, original);
    check!(eq; witness::capture(&ExportDataset::reopen_frozen_with_digest(&current_path, &current_digest)?)?, current);
    check!(ExportDataset::reopen_frozen_with_digest(&original_path, &current_digest).is_err());
    Ok(())
}

fn request(fixture: &Fixture, case_id: &str) -> TestResult<LocalIdentitySplitV1> {
    let canonical = fixture.births.first().ok_or("first birth")?.public_id();
    let head = fixture.writer.head(&fixture.store)?;
    let inventory = fixture
        .writer
        .inspect_local_adjudication(&fixture.store, canonical)?;
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
    #[derive(Serialize)]
    struct Envelope<'a> {
        schema: &'static str,
        actor: &'static str,
        prepared: &'a census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV2,
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
