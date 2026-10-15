use super::*;
use crate::public_identity_tests::support::retained_state;
use crate::{Store, Table};

#[test]
fn manual_inventory_is_read_only_and_matches_the_staged_context_identity() -> TestResult {
    let fixture = merged()?;
    let extra = crate::public_identity_tests::support::subject("manual-inspect-unbound", "9988")?;
    fixture.store.append(Table::Athletes, &extra)?;
    let before = retained_state(&fixture.store)?;
    let sequence = fixture.store.snapshot().sequence();
    let canonical = fixture.births.first().ok_or("birth")?.public_id();
    let inventory = fixture
        .writer
        .inspect_local_adjudication(&fixture.store, canonical)?;
    assert!(!inventory.claims().is_empty());
    assert_eq!(retained_state(&fixture.store)?, before);
    assert_eq!(fixture.store.snapshot().sequence(), sequence);
    let repeated = fixture
        .writer
        .inspect_local_adjudication(&fixture.store, canonical)?;
    assert_eq!(repeated.authority_context(), inventory.authority_context());
    assert_eq!(repeated.claims(), inventory.claims());
    assert_eq!(retained_state(&fixture.store)?, before);
    let inputs = super::super::Inputs::capture(&fixture.store.snapshot())?;
    let staged = inputs.stage(&fixture.store, &fixture.store.store_identity()?)?;
    assert_eq!(staged.object_id(), inventory.authority_context());
    Ok(())
}

#[test]
fn approved_manual_split_commits_reopens_and_frozen_replay_keeps_original_evidence() -> TestResult {
    let fixture = merged()?;
    let before =
        serde_json::to_value(super::super::Inputs::capture(&fixture.store.snapshot())?.athletes)?;
    let accepted_rows = fixture.store.count(Table::AthleteIdentityDecisions)?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-durable-manual",
        request(&fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let (bytes, digest) = approval(&prepared)?;
    let outcome =
        fixture
            .writer
            .apply_local_adjudication(&fixture.store, prepared, bytes, digest.clone())?;
    assert!(!outcome.replayed());
    assert_eq!(outcome.introduced_public_ids().len(), 2);
    assert_eq!(
        fixture.store.count(Table::AthleteIdentityDecisions)?,
        accepted_rows
    );
    let old = fixture.births.first().ok_or("birth")?.public_id().clone();
    let accepted = fixture
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("snapshot")?;
    assert!(matches!(
        accepted.resolve_public(&old)?,
        PublicResolution::Ambiguous { .. }
    ));
    let mut expected_ids = fixture
        .births
        .iter()
        .map(|birth| birth.public_id().clone())
        .chain(outcome.introduced_public_ids().iter().cloned())
        .collect::<Vec<_>>();
    expected_ids.sort();
    assert_eq!(
        accepted.public_ids().cloned().collect::<Vec<_>>(),
        expected_ids
    );
    let expected_inventory = public_inventory(&accepted)?;
    let [_, second_birth, unrelated_birth] = &fixture.births;
    assert!(matches!(
        accepted.resolve_public(second_birth.public_id())?,
        PublicResolution::RevokedRedirect { .. }
    ));
    assert!(matches!(
        accepted.resolve_public(unrelated_birth.public_id())?,
        PublicResolution::Canonical(_)
    ));
    for introduced in outcome.introduced_public_ids() {
        assert!(matches!(
            accepted.resolve_public(introduced)?,
            PublicResolution::Canonical(_)
        ));
    }
    let state = retained_state(&fixture.store)?;
    let repeated = fixture
        .writer
        .replay_local_adjudication(&fixture.store, "synthetic-durable-manual", &digest)?
        .ok_or("repeat")?;
    assert!(repeated.replayed());
    assert_eq!(repeated.head(), outcome.head());
    assert_eq!(retained_state(&fixture.store)?, state);
    let frozen = fixture.store.snapshot().freeze_public_identity_v1()?;
    let root = fixture.directory.path().join("identity");
    let ReviewedFixture {
        directory,
        store,
        writer,
        ..
    } = fixture;
    drop(writer);
    drop(store);
    let (reopened, writer) = Store::open_public_identity_qualification_v2(&root)?.into_parts();
    assert_eq!(writer.head(&reopened)?, *outcome.head());
    let reopened_snapshot = reopened
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("reopened snapshot")?;
    assert_eq!(public_inventory(&reopened_snapshot)?, expected_inventory);
    let after =
        serde_json::to_value(super::super::Inputs::capture(&reopened.snapshot())?.athletes)?;
    assert_eq!(after, before);
    assert_eq!(
        reopened.count(Table::AthleteIdentityDecisions)?,
        accepted_rows
    );
    let replayed = frozen.replay()?;
    assert_eq!(replayed.head(), outcome.head());
    assert_eq!(
        replayed.resolve_public(&old)?,
        accepted.resolve_public(&old)?
    );
    assert_eq!(public_inventory(&replayed)?, expected_inventory);
    drop(writer);
    drop(reopened);
    drop(directory);
    Ok(())
}

fn public_inventory(
    snapshot: &crate::AcceptedPublicIdentitySnapshot,
) -> TestResult<
    Vec<(
        census_domain::public_identity_history::PublicAthleteId,
        PublicResolution,
    )>,
> {
    snapshot
        .public_ids()
        .map(|id| Ok((id.clone(), snapshot.resolve_public(id)?)))
        .collect()
}

#[test]
fn persisted_manual_decoder_refuses_unknown_fields_and_oversized_disposition_inventory(
) -> TestResult {
    let fixture = merged()?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-manual-decode",
        request(&fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let original = serde_json::to_value(prepared.record())?;
    let mut unknown = original.clone();
    unknown
        .as_object_mut()
        .ok_or("object")?
        .insert("actor_override".into(), serde_json::json!("model"));
    assert!(serde_json::from_value::<census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV1>(unknown).is_err());
    let mut oversized = original;
    let request = oversized
        .get_mut("request")
        .ok_or("request")?
        .as_object_mut()
        .ok_or("request object")?;
    let one = request
        .get("dispositions")
        .and_then(|value| value.as_array())
        .and_then(|rows| rows.first())
        .ok_or("claim")?
        .clone();
    request.insert(
        "dispositions".into(),
        serde_json::Value::Array(vec![one; 4097]),
    );
    let result = serde_json::from_value::<census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV1>(oversized);
    assert!(result.is_err());
    Ok(())
}

#[test]
fn frozen_manual_kind_cannot_carry_a_forged_accepted_decision_row() -> TestResult {
    let fixture = merged()?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-manual-kind",
        request(&fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let (bytes, digest) = approval(&prepared)?;
    fixture
        .writer
        .apply_local_adjudication(&fixture.store, prepared, bytes, digest)?;
    let frozen = fixture.store.snapshot().freeze_public_identity_v1()?;
    let mut wire: serde_json::Value = serde_json::from_slice(frozen.bytes())?;
    let operations = wire
        .get_mut("operations")
        .and_then(|value| value.as_array_mut())
        .ok_or("operations")?;
    let last = operations
        .last_mut()
        .and_then(|value| value.as_object_mut())
        .ok_or("last operation")?;
    assert!(!last.contains_key("decision"));
    last.insert("decision".into(), serde_json::json!("{}"));
    let changed = serde_json::to_vec(&wire)?;
    let digest = format!("{:x}", Sha256::digest(&changed));
    let result = crate::replay_frozen_public_identity_v1(&changed, &digest);
    assert!(matches!(
        result,
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "frozen decision material kind"
        )))
    ));
    Ok(())
}

#[test]
fn manual_witness_roundtrip_preserves_the_exact_record_and_legacy_witness_ceiling() -> TestResult {
    let fixture = merged()?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-manual-wire",
        request(&fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let expected = serde_json::to_value(prepared.record())?;
    let (bytes, digest) = approval(&prepared)?;
    let outcome =
        fixture
            .writer
            .apply_local_adjudication(&fixture.store, prepared, bytes, digest)?;
    let key = crate::public_identity::witness::key(outcome.head().sequence);
    let raw = fixture.store.journal.get(&key)?.ok_or("witness")?;
    let held = crate::public_identity::operation::HeldWitness::decode(&key, &raw)?;
    let crate::public_identity::operation::HeldWitness::Manual(manual) = held else {
        return Err("wrong witness kind".into());
    };
    assert_eq!(serde_json::to_value(&manual.adjudication)?, expected);
    assert_eq!(serde_json::to_vec(&manual)?, raw.to_vec());
    let mut oversized = br#"{"version":3,"padding":""#.to_vec();
    oversized.extend(vec![b'x'; 65_536]);
    oversized.extend_from_slice(b"\"}");
    let result = crate::public_identity::operation::HeldWitness::decode("legacy", &oversized);
    assert!(matches!(
        result,
        Err(StoreError::RowTooLarge { max: 65_536, .. })
    ));
    Ok(())
}

#[test]
fn superseded_exact_case_cannot_automatically_remerge_manual_successors() -> TestResult {
    let fixture = merged()?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-manual-fence",
        request(&fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let (bytes, digest) = approval(&prepared)?;
    let split = fixture
        .writer
        .apply_local_adjudication(&fixture.store, prepared, bytes, digest)?;
    let ids = split.introduced_public_ids();
    let request = crate::ReviewedIdentityRequestV1::Merge {
        case_id: fixture.request.case_id().into(),
        canonical: ids.first().ok_or("successor")?.clone(),
        members: ids.to_vec(),
    };
    let before = retained_state(&fixture.store)?;
    let result = fixture.writer.apply_reviewed_identity(
        &fixture.store,
        "synthetic-forbidden-remerge",
        &request,
        fixture.store.snapshot().revision(),
        split.head(),
    );
    assert!(matches!(
        result,
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "reviewed authority superseded by local adjudication"
        )))
    ));
    assert_eq!(retained_state(&fixture.store)?, before);
    assert_eq!(fixture.writer.head(&fixture.store)?, *split.head());
    Ok(())
}

#[test]
fn frozen_source_kind_still_requires_accepted_decision_material() -> TestResult {
    let fixture = merged()?;
    let frozen = fixture.store.snapshot().freeze_public_identity_v1()?;
    let mut wire: serde_json::Value = serde_json::from_slice(frozen.bytes())?;
    let operation = wire
        .get_mut("operations")
        .and_then(|value| value.as_array_mut())
        .and_then(|rows| rows.first_mut())
        .and_then(|value| value.as_object_mut())
        .ok_or("operation")?;
    operation.remove("decision").ok_or("accepted decision")?;
    let bytes = serde_json::to_vec(&wire)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let result = crate::replay_frozen_public_identity_v1(&bytes, &digest);
    assert!(matches!(
        result,
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "frozen decision material kind"
        )))
    ));
    Ok(())
}

#[test]
fn witness_version_byte_budgets_accept_exact_boundary_and_refuse_one_more() -> TestResult {
    let fixture = merged()?;
    let prepared = fixture.writer.prepare_local_adjudication(
        &fixture.store,
        "synthetic-manual-byte-bound",
        request(&fixture)?,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    let (bytes, digest) = approval(&prepared)?;
    let outcome =
        fixture
            .writer
            .apply_local_adjudication(&fixture.store, prepared, bytes, digest)?;
    for (sequence, ceiling) in [(1, 65_536), (outcome.head().sequence, 1024 * 1024)] {
        let key = crate::public_identity::witness::key(sequence);
        let raw = fixture.store.journal.get(&key)?.ok_or("witness")?;
        let mut padded = raw.to_vec();
        assert!(padded.len() <= ceiling);
        padded.resize(ceiling, b' ');
        crate::public_identity::operation::HeldWitness::decode(&key, &padded)?;
        padded.push(b' ');
        let result = crate::public_identity::operation::HeldWitness::decode(&key, &padded);
        assert!(matches!(result, Err(StoreError::RowTooLarge { max, .. }) if max == ceiling));
    }
    Ok(())
}

#[test]
fn material_fence_ignores_case_labels_and_model_metadata_but_binds_facts() -> TestResult {
    let fixture = merged()?;
    let inputs = super::super::Inputs::capture(&fixture.store.snapshot())?;
    let members: Vec<_> = fixture.candidates.iter().take(2).cloned().collect();
    let original = crate::public_identity::authority::identity_material(&inputs, &members)?;
    let mut relabeled = super::super::Inputs::capture(&fixture.store.snapshot())?;
    for case in &mut relabeled.cases {
        case.subject = "Changed display label".into();
        case.detail = "Another model rerun description".into();
    }
    for verdict in &mut relabeled.verdicts {
        verdict.reviewer = "Another synthetic model".into();
        verdict.rationale = "Same facts, new wording".into();
    }
    assert_eq!(
        crate::public_identity::authority::identity_material(&relabeled, &members)?,
        original
    );
    let athlete = relabeled.athletes.first_mut().ok_or("athlete")?;
    athlete.canonical_name = "Changed synthetic source name".into();
    assert_ne!(
        crate::public_identity::authority::identity_material(&relabeled, &members)?,
        original
    );
    Ok(())
}
