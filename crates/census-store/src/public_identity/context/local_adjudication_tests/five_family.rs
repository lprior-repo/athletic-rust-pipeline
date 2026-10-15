use super::*;
mod oracle;
use crate::public_identity::{context::reviewed_fixture::review, graph_witness::HeldGraphWitness};
use crate::{
    AcceptedPublicIdentitySnapshot, FrozenPublicIdentityBundleV2,
    SourceCandidateMembershipRequestV1, Store, Table,
};
use census_domain::model::{CanonicalAthlete, CanonicalPerformance, Mark, RelayMember, SchoolId};
use census_domain::public_identity_history::local_membership_withholding::{
    LocalMembershipWithholdV1, MembershipAttachmentV1,
};

#[test]
fn five_graph_families_preserve_later_page_results_and_replay_every_checkpoint_offline(
) -> TestResult {
    let fixture = ready_for_merge()?;
    results::install(&fixture)?;
    let late_keys = install_later_page_variants(&fixture)?;
    let mut checkpoints = vec![checkpoint(&fixture.store)?];
    let facts = results::retained_facts(&fixture.store)?;
    let merged = fixture.writer.apply_reviewed_identity(
        &fixture.store,
        "five-reviewed",
        &fixture.request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    check!(eq; merged.head().sequence, 4);
    check!(eq; results::retained_facts(&fixture.store)?, facts);
    checkpoints.push(checkpoint(&fixture.store)?);
    let attachment = membership(&fixture)?;
    let facts = results::retained_facts(&fixture.store)?;
    let attached = fixture.writer.extend_source_candidate(
        &fixture.store,
        "five-membership",
        &attachment,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    check!(eq; attached.head().sequence, 5);
    check!(eq; attached.public_id(), &attachment.owner);
    check!(attached.introduced_public_ids().is_empty());
    check!(eq; results::retained_facts(&fixture.store)?, facts);
    checkpoints.push(checkpoint(&fixture.store)?);
    let inventory = fixture.writer.inspect_membership_withholding(
        &fixture.store,
        "five-withhold",
        &attachment.owner,
    )?;
    let held = witness(&fixture.store, attached.head().sequence)?;
    let id = oracle::withholding_claims(
        &fixture,
        &attachment,
        &attached.head().digest,
        inventory.claims(),
    )?;
    check!(eq; id, format!("source_membership_v1_{}", held.request_digest));
    let withhold = LocalMembershipWithholdV1 {
        case_id: attachment.case_id.clone(),
        canonical: attachment.owner.clone(),
        attachment: MembershipAttachmentV1 {
            sequence: attached.head().sequence,
            digest: attached.head().digest.clone(),
            membership_id: id.clone(),
        },
        expected_withheld_memberships: vec![id],
        affected_record_keys: inventory.affected_record_keys().to_vec(),
        rationale: "Withdraw this retained source attachment without changing its source facts"
            .into(),
    };
    let mut expected_withheld_keys = results::unrelated_keys(&fixture)?;
    expected_withheld_keys.push(format!("athletes:{}", attachment.candidate));
    expected_withheld_keys.push(format!(
        "performances:{}",
        results::performance(&fixture.candidates[2].cast(), "raw-variant")?.id
    ));
    expected_withheld_keys.push(format!(
        "relay_results:{}",
        results::relay(&fixture.candidates[2].cast(), "raw-variant").id
    ));
    expected_withheld_keys.extend(late_keys.clone());
    expected_withheld_keys.sort();
    check!(eq; withhold.affected_record_keys, expected_withheld_keys);
    let prepared = fixture.writer.prepare_membership_withholding(
        &fixture.store,
        "five-withhold",
        withhold.clone(),
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    check!(eq; prepared.record().dispositions.iter().map(|row| &row.claim).collect::<Vec<_>>(), inventory.claims().iter().collect::<Vec<_>>());
    for disposition in &prepared.record().dispositions {
        let expected = if disposition.claim.section == census_domain::public_identity_history::local_adjudication::RetainedIdentitySectionV1::CandidateMembership {
            ClaimTreatmentV1::RejectIdentityCorroboration
        } else { ClaimTreatmentV1::RetainAsContext };
        check!(eq; disposition.treatment, expected);
    }
    let bytes = prepared.approval_bytes()?;
    let digest = prepared.approval_digest()?;
    let withheld =
        fixture
            .writer
            .apply_membership_withholding(&fixture.store, prepared, bytes, digest)?;
    check!(eq; withheld.head().sequence, 6);
    check!(eq; withheld.withheld_memberships(), withhold.expected_withheld_memberships.as_slice());
    check!(withheld.introduced_public_ids().is_empty());
    check!(eq; results::retained_facts(&fixture.store)?, facts);
    checkpoints.push(checkpoint(&fixture.store)?);
    let request = request(&fixture)?;
    oracle::claims(&fixture, &request)?;
    let mut expected_manual_keys = results::expected_keys(&fixture)?;
    expected_manual_keys.extend(late_keys.clone());
    expected_manual_keys.sort();
    check!(eq; request.affected_record_keys, expected_manual_keys);
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "five-manual",
        request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    assert_later_page_members(&fixture, prepared.graph())?;
    let bytes = prepared.approval_bytes()?;
    let digest = prepared.approval_digest()?;
    let split =
        fixture
            .writer
            .apply_local_adjudication(&fixture.store, prepared, bytes, digest.clone())?;
    check!(eq; split.head().sequence, 7);
    check!(eq; split.introduced_public_ids().len(), 2);
    check!(eq; results::retained_facts(&fixture.store)?, facts);
    checkpoints.push(checkpoint(&fixture.store)?);
    let versions = (1..=7)
        .map(|sequence| Ok(witness(&fixture.store, sequence)?.version))
        .collect::<TestResult<Vec<_>>>()?;
    check!(eq; versions, vec![8, 8, 8, 9, 11, 12, 10]);
    let current = fixture
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("snapshot")?;
    check!(matches!(
        current.resolve_public(fixture.births.first().ok_or("birth")?.public_id())?,
        PublicResolution::Ambiguous { .. }
    ));
    check!(eq; current.resolve_candidate(&attachment.candidate)?, PublicResolution::Unresolved);
    check!(
        matches!(current.resolve_public(&attachment.owner)?, PublicResolution::Canonical(owner) if owner.public_id == attachment.owner)
    );
    let before = crate::public_identity_tests::support::retained_state(&fixture.store)?;
    let retry = fixture
        .writer
        .replay_local_adjudication(&fixture.store, "five-manual", &digest)?
        .ok_or("manual replay")?;
    check!(retry.replayed());
    check!(eq; retry.head(), split.head());
    check!(eq; crate::public_identity_tests::support::retained_state(&fixture.store)?, before);
    fixture.store.append(
        Table::Performances,
        &results::performance(
            &fixture.candidates.first().ok_or("member")?.cast(),
            "five-live-growth",
        )?,
    )?;
    let grown = fixture
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("grown snapshot")?;
    check!(grown.snapshot_sequence() > current.snapshot_sequence());
    check!(eq; grown.history_records(), current.history_records());
    check!(eq; grown.public_ids().collect::<Vec<_>>(), current.public_ids().collect::<Vec<_>>());
    for id in current.public_ids() {
        check!(eq; grown.resolve_public(id)?, current.resolve_public(id)?);
    }
    checkpoints.push(checkpoint(&fixture.store)?);
    oracle::checkpoints(
        &fixture,
        &checkpoints,
        &attachment,
        &expected_manual_keys,
        &expected_withheld_keys,
    )?;
    let path = fixture.directory.path().join("identity");
    drop(fixture.writer);
    drop(fixture.store);
    let (store, writer) = Store::open_public_identity_qualification_v3(&path)?.into_parts();
    grown.require_same_projection(
        &store
            .snapshot()
            .accepted_public_identity_snapshot()?
            .ok_or("reopened snapshot")?,
    )?;
    check!(eq; writer.head(&store)?, *split.head());
    drop(writer);
    drop(store);
    drop(fixture.directory);
    for (accepted, frozen) in checkpoints {
        accepted.require_same_projection(&frozen.replay()?)?;
    }
    Ok(())
}

fn checkpoint(
    store: &Store,
) -> TestResult<(AcceptedPublicIdentitySnapshot, FrozenPublicIdentityBundleV2)> {
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("snapshot")?;
    let frozen = snapshot.freeze_public_identity_v2()?;
    accepted.require_same_projection(&frozen.replay()?)?;
    Ok((accepted, frozen))
}

fn witness(store: &Store, sequence: u64) -> TestResult<HeldGraphWitness> {
    let key = crate::public_identity::witness::key(sequence);
    let raw = store.journal.get(&key)?.ok_or("graph witness")?;
    Ok(HeldGraphWitness::decode(&key, &raw)?)
}

fn membership(fixture: &ReviewedFixture) -> TestResult<SourceCandidateMembershipRequestV1> {
    let first = fixture
        .store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .into_iter()
        .find(|row| row.id.as_str() == fixture.candidates[2].as_str())
        .ok_or("anchor athlete")?;
    let mut second = CanonicalAthlete::new(
        &SchoolId::mint("sch", &["five-family-second-school"]),
        &first.canonical_name,
        first.grad_year,
        first.gender,
        first.source.clone().ok_or("primary")?,
    )?;
    second.evidence = first.evidence.clone();
    fixture.store.append(Table::Athletes, &second)?;
    let (case, verdict) = review(
        &fixture.store,
        &[first.id.cast(), second.id.cast()],
        "same_person",
    )?;
    let mut cases = fixture
        .store
        .scan::<census_domain::model::ReviewCase>(Table::ReviewCases)?;
    let mut verdicts = fixture
        .store
        .scan::<census_domain::model::ReviewVerdictRecord>(Table::IdentityVerdicts)?;
    cases.push(case.clone());
    verdicts.push(verdict);
    fixture.store.replace_many(Table::ReviewCases, &cases)?;
    fixture
        .store
        .replace_many(Table::IdentityVerdicts, &verdicts)?;
    Ok(SourceCandidateMembershipRequestV1 {
        case_id: case.id,
        owner: fixture.births[2].public_id().clone(),
        anchor_binding_id: binding_id(&fixture.store, fixture.births[2].head().sequence)?,
        candidate: second.id.cast(),
    })
}

fn install_later_page_variants(fixture: &ReviewedFixture) -> TestResult<Vec<String>> {
    let unrelated = fixture.candidates[2].cast();
    let base_performance = results::performance(&unrelated, "five-later-page-variant")?;
    let base_relay = results::relay(&unrelated, "five-later-page-variant");
    let mut performances = Vec::new();
    let mut relays = Vec::new();
    for ordinal in 0..520 {
        let mut performance = base_performance.clone();
        performance.mark = Mark::Raw(format!("retained-variant-{ordinal}"));
        performances.push(performance);
        let mut relay = base_relay.clone();
        relay.members = vec![
            RelayMember::new(1, format!("unrelated retained variant {ordinal}"))
                .with_athlete(unrelated.clone()),
        ];
        relays.push(relay);
    }
    let mut performance = base_performance.clone();
    performance.athlete = fixture.candidates[0].cast();
    performances.push(performance);
    let mut relay = base_relay.clone();
    relay.members = vec![
        RelayMember::new(1, "later retained member").with_athlete(fixture.candidates[1].cast())
    ];
    relays.push(relay);
    fixture
        .store
        .append_many(Table::Performances, &performances)?;
    fixture.store.append_many(Table::RelayResults, &relays)?;
    let mut keys = vec![
        format!("performances:{}", base_performance.id),
        format!("relay_results:{}", base_relay.id),
    ];
    keys.sort();
    Ok(keys)
}

fn assert_later_page_members(
    fixture: &ReviewedFixture,
    reference: &crate::public_identity::graph_reference::GraphAuthorityReferenceV1,
) -> TestResult {
    use crate::public_identity::paged_generation::{AuthorityTableV2, VerifiedRawGraph};
    let store = &fixture.store;
    let snapshot = store.snapshot();
    let graph = VerifiedRawGraph::verify(&snapshot, reference.capture(), reference.seal())?;
    let mut performances = Vec::new();
    let mut relays = Vec::new();
    graph.for_each_physical::<CanonicalPerformance>(
        AuthorityTableV2::Performances,
        super::super::BUDGET,
        |row, key, raw, ordinal, _| {
            performances.push((row, key.to_vec(), raw.to_vec(), ordinal));
            Ok(())
        },
    )?;
    graph.for_each_physical::<census_domain::model::RelayResult>(
        AuthorityTableV2::RelayResults,
        super::super::BUDGET,
        |row, key, raw, ordinal, _| {
            relays.push((row, key.to_vec(), raw.to_vec(), ordinal));
            Ok(())
        },
    )?;
    check!(eq; performances.len(), 526);
    check!(eq; relays.len(), 526);
    for (ordinal, (_, key, raw, held)) in performances.iter().enumerate() {
        check!(eq; *held, u64::try_from(ordinal)?);
        check!(!key.is_empty());
        check!(!raw.is_empty());
    }
    for (ordinal, (_, key, raw, held)) in relays.iter().enumerate() {
        check!(eq; *held, u64::try_from(ordinal)?);
        check!(!key.is_empty());
        check!(!raw.is_empty());
    }
    check!(eq; performances.iter().map(|(_, key, raw, ordinal)| (key.clone(), raw.clone(), *ordinal)).collect::<Vec<_>>(), oracle::physical(store, Table::Performances)?);
    check!(eq; relays.iter().map(|(_, key, raw, ordinal)| (key.clone(), raw.clone(), *ordinal)).collect::<Vec<_>>(), oracle::physical(store, Table::RelayResults)?);
    let performance_id =
        results::performance(&fixture.candidates[2].cast(), "five-later-page-variant")?.id;
    let relay_id = results::relay(&fixture.candidates[2].cast(), "five-later-page-variant").id;
    check!(performances
        .iter()
        .any(|(row, _, _, ordinal)| row.id == performance_id
            && *ordinal >= 512
            && row.athlete.as_str() == fixture.candidates[0].as_str()));
    check!(relays.iter().any(|(row, _, _, ordinal)| row.id == relay_id
        && *ordinal >= 512
        && row.members.iter().any(|member| member
            .athlete
            .as_ref()
            .is_some_and(|athlete| athlete.as_str() == fixture.candidates[1].as_str()))));
    check!(eq; serde_json::to_value(performances.iter().map(|(row, _, _, _)| row).collect::<Vec<_>>())?, serde_json::to_value(results::raw::<CanonicalPerformance>(store, Table::Performances)?)?);
    check!(eq; serde_json::to_value(relays.iter().map(|(row, _, _, _)| row).collect::<Vec<_>>())?, serde_json::to_value(results::raw::<census_domain::model::RelayResult>(store, Table::RelayResults)?)?);

    Ok(())
}
