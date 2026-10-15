use super::test_fixture::*;
use super::{read, wire, Inputs, BUDGET};
use crate::journal::{object_chunk_key, object_manifest_key};
use crate::public_identity::authority::accept_inputs;
use crate::public_identity_tests::support::{fixture, retained_state, TestResult};
use crate::{PublicIdentityError, Store, StoreError, Table};
use census_domain::model::{
    AppliedAthleteIdentity, ReviewCase, SourceObservation, ATHLETE_IDENTITY_FAMILY,
};
use serde_json::json;

#[test]
fn full_raw_context_root_ignores_operation_candidate_and_decision_time() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    install_inputs(&store, &full_inputs()?)?;
    let inputs = Inputs::capture(&store.snapshot())?;
    let first = &inputs
        .athletes
        .iter()
        .find(|athlete| {
            athlete
                .source
                .as_ref()
                .is_some_and(|source| source.id == "9201")
        })
        .ok_or("first athlete")?
        .id
        .cast();
    let second = &inputs
        .athletes
        .iter()
        .find(|athlete| {
            athlete
                .source
                .as_ref()
                .is_some_and(|source| source.id == "9202")
        })
        .ok_or("second athlete")?
        .id
        .cast();
    let lineage = writer.store_identity();
    let staged = inputs.stage(&store, lineage)?;
    let before = retained_state(&store)?;
    let (early, _) = accept_inputs(&inputs, first, "2026-10-14T00:00:00Z")?;
    let (late, _) = accept_inputs(&inputs, first, "2026-10-15T23:59:59Z")?;
    let (other, _) = accept_inputs(&inputs, second, "2027-01-01T00:00:00Z")?;
    check!(eq; early.record().observed_at, "2026-10-14T00:00:00Z");
    check!(eq; late.record().observed_at, "2026-10-15T23:59:59Z");
    check!(ne; early.record().members, other.record().members);
    check!(eq; inputs.stage(&store, lineage)?, staged);
    check!(eq; retained_state(&store)?, before);
    let head = writer.head(&store)?;
    let committed = writer.establish_source_bound(
        &store,
        "context-first-operation",
        first,
        store.snapshot().revision(),
        &head,
    )?;
    let next = writer.establish_source_bound(
        &store,
        "context-second-operation",
        second,
        store.snapshot().revision(),
        committed.head(),
    )?;
    check!(eq; next.head().sequence, 2);
    for sequence in [1, 2] {
        let reference = graph_reference(&store, sequence)?;
        check!(ne; reference.seal().root().as_str(), staged.object_id());
        check!(eq; exact_inputs(&graph_inputs(&store, &reference)?)?, exact_inputs(&inputs)?);
    }
    check!(eq; read_inputs(&store, staged.object_id())?.cases.len(), 1);
    check!(eq; read_inputs(&store, staged.object_id())?.verdicts.len(), 1);
    Ok(())
}

#[test]
fn retained_context_preserves_original_rows_before_authority_enrichment() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let inputs = full_inputs()?;
    let before = exact_inputs(&inputs)?;
    let candidate = inputs.athletes.first().ok_or("first athlete")?.id.cast();
    let (accepted, _) = accept_inputs(&inputs, &candidate, "2026-10-14T00:00:00Z")?;
    check!(eq; accepted.record().members.first().ok_or("accepted member")?.subject, candidate);
    check!(eq; exact_inputs(&inputs)?, before);
    let mut enriched = inputs.athletes.clone();
    crate::read::enrich_retained_observations(&mut enriched, &inputs.observations, BUDGET)?;
    check!(eq; enriched.first().ok_or("enriched athlete")?.observed_grades, vec![grade()?]);
    let staged = inputs.stage(&store, writer.store_identity())?;
    let restored = read_inputs(&store, staged.object_id())?;
    check!(eq; exact_inputs(&restored)?, before);
    check!(eq; restored.athletes.first().ok_or("original athlete")?.observed_grades.len(), 0);
    let reviewed = restored
        .athletes
        .get(2)
        .ok_or("reviewed athlete")?
        .id
        .cast();
    match accept_inputs(&restored, &reviewed, "2026-10-14T00:00:00Z") {
        Err(StoreError::PublicIdentity(PublicIdentityError::NotSourceBound)) => {}
        Err(error) => return Err(format!("unexpected retained review error: {error:?}").into()),
        Ok(_) => return Err("retained review context was dropped".into()),
    }
    read::verify(
        &store.db.snapshot(),
        &store.journal,
        staged.object_id(),
        accepted.record(),
    )?;
    Ok(())
}

#[test]
fn each_raw_authority_section_changes_the_retained_root() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let inputs = full_inputs()?;
    let original = inputs.stage(&store, writer.store_identity())?;
    for section in 0..4 {
        let mut changed = full_inputs()?;
        match section {
            0 => changed
                .athletes
                .first_mut()
                .ok_or("athlete")?
                .evidence
                .clear(),
            1 => {
                let SourceObservation::Athlete(row) =
                    changed.observations.first_mut().ok_or("observation")?
                else {
                    return Err("fixture observation is not an athlete".into());
                };
                row.source_row_key = "changed-retained-row".into();
            }
            2 => changed
                .cases
                .first_mut()
                .ok_or("case")?
                .detail
                .push_str(" changed"),
            3 => changed
                .verdicts
                .first_mut()
                .ok_or("verdict")?
                .rationale
                .push_str(" changed"),
            _ => return Err("unknown fixture section".into()),
        }
        let staged = changed.stage(&store, writer.store_identity())?;
        check!(ne; staged.object_id(), original.object_id());
        check!(eq; exact_inputs(&read_inputs(&store, staged.object_id())?)?, exact_inputs(&changed)?);
        check!(eq; exact_inputs(&read_inputs(&store, original.object_id())?)?, exact_inputs(&inputs)?);
    }
    Ok(())
}

#[test]
fn foreign_store_lineage_changes_root_and_cannot_replay_locally() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let (_foreign_directory, _foreign_store, foreign_writer) = fixture()?;
    let inputs = full_inputs()?;
    let local = inputs.stage(&store, writer.store_identity())?;
    let foreign = inputs.stage(&store, foreign_writer.store_identity())?;
    check!(ne; local.object_id(), foreign.object_id());
    refusal(
        read_inputs(&store, foreign.object_id()),
        "unsupported or foreign retained authority context",
    )?;
    check!(eq; exact_inputs(&read_inputs(&store, local.object_id())?)?, exact_inputs(&inputs)?);
    Ok(())
}

#[test]
fn unsupported_context_version_and_policy_revisions_fail_closed() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let staged = full_inputs()?.stage(&store, writer.store_identity())?;
    for field in ["version", "authority_policy", "enrichment_policy"] {
        let mut rows = records(&store, staged.object_id())?;
        *header_mut(&mut rows)?
            .get_mut(field)
            .ok_or("missing header field")? = json!(2);
        let changed = stage_records(&store, &rows)?;
        check!(ne; changed.object_id(), staged.object_id());
        refusal(
            read_inputs(&store, changed.object_id()),
            "unsupported or foreign retained authority context",
        )?;
    }
    Ok(())
}

#[test]
fn retained_header_counts_must_cover_exactly_the_declared_sections() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let staged = full_inputs()?.stage(&store, writer.store_identity())?;
    for (counts, detail) in [
        ([2, 1, 1, 1], "retained authority context count mismatch"),
        ([4, 1, 1, 1], "retained authority context count mismatch"),
        (
            [8193, 0, 0, 0],
            "retained authority section exceeds its row bound",
        ),
    ] {
        let mut rows = records(&store, staged.object_id())?;
        *header_mut(&mut rows)?.get_mut("counts").ok_or("counts")? = json!(counts);
        let changed = stage_records(&store, &rows)?;
        refusal(read_inputs(&store, changed.object_id()), detail)?;
    }
    Ok(())
}

#[test]
fn retained_section_tags_and_envelope_positions_are_not_interchangeable() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let staged = full_inputs()?.stage(&store, writer.store_identity())?;
    for (field, value, detail) in [
        (
            "/payload/kind",
            "verdict",
            "retained authority section order mismatch",
        ),
        (
            "/key",
            "athlete:00000000000000000001",
            "retained authority record envelope mismatch",
        ),
        (
            "/at",
            "2026-10-14",
            "retained authority record envelope mismatch",
        ),
    ] {
        let mut rows = records(&store, staged.object_id())?;
        *rows
            .get_mut(1)
            .and_then(|row| row.pointer_mut(field))
            .ok_or("athlete envelope")? = json!(value);
        let changed = stage_records(&store, &rows)?;
        refusal(read_inputs(&store, changed.object_id()), detail)?;
    }
    Ok(())
}

#[test]
fn missing_and_corrupt_context_chunks_are_refused_without_repair() -> TestResult {
    for remove in [false, true] {
        let (_directory, store, writer) = fixture()?;
        let staged = full_inputs()?.stage(&store, writer.store_identity())?;
        let key = object_chunk_key(staged.object_id(), 0)?;
        if remove {
            let mut batch = store.db.batch();
            batch.remove(&store.journal, &key);
            batch.commit()?;
        } else {
            let raw = store.journal.get(&key)?.ok_or("chunk")?;
            let mut chunk: serde_json::Value = serde_json::from_slice(&raw)?;
            *chunk.get_mut("digest").ok_or("chunk digest")? = json!("0".repeat(64));
            write_journal(&store, &key, &serde_json::to_vec(&chunk)?)?;
        }
        let before = retained_state(&store)?;
        match read_inputs(&store, staged.object_id()) {
            Err(StoreError::JournalChunkMissing { phase, key, index }) if remove => {
                check!(eq; phase, "authority_context_v1");
                check!(eq; key, staged.object_id());
                check!(eq; index, 0);
            }
            Err(StoreError::JournalChunkDigest {
                phase,
                key,
                index,
                declared,
                computed,
            }) if !remove => {
                check!(eq; phase, "authority_context_v1");
                check!(eq; key, staged.object_id());
                check!(eq; index, 0);
                check!(eq; declared, "0".repeat(64));
                check!(ne; computed, declared);
            }
            Err(error) => {
                return Err(format!("unexpected context corruption error: {error:?}").into())
            }
            Ok(_) => return Err("corrupt context was accepted".into()),
        }
        check!(eq; retained_state(&store)?, before);
    }
    Ok(())
}

#[test]
fn manifest_reference_counts_cannot_hide_missing_or_surplus_records() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let staged = full_inputs()?.stage(&store, writer.store_identity())?;
    let key = object_manifest_key(staged.object_id())?;
    let original = store.journal.get(&key)?.ok_or("manifest")?;
    for count in [6, 8] {
        let mut manifest: serde_json::Value = serde_json::from_slice(&original)?;
        *manifest.get_mut("records").ok_or("manifest records")? = json!(count);
        write_journal(&store, &key, &serde_json::to_vec(&manifest)?)?;
        match read_inputs(&store, staged.object_id()) {
            Err(StoreError::JournalDescriptor { phase, key, detail }) => {
                check!(eq; phase, "authority_context_v1");
                check!(eq; key, staged.object_id());
                check!(eq; detail, "index page holds an unexpected number of entries");
            }
            Err(error) => return Err(format!("unexpected reference-count error: {error:?}").into()),
            Ok(_) => return Err("incorrect reference count was accepted".into()),
        }
    }
    Ok(())
}

#[test]
fn reopened_ledger_replays_retained_decision_after_live_review_changes() -> TestResult {
    let (directory, store, writer) = fixture()?;
    install_inputs(&store, &full_inputs()?)?;
    let original = Inputs::capture(&store.snapshot())?;
    let athlete = original
        .athletes
        .iter()
        .find(|athlete| {
            athlete
                .source
                .as_ref()
                .is_some_and(|source| source.id == "9201")
        })
        .ok_or("first candidate")?;
    let candidate = athlete.id.cast();
    let head = writer.head(&store)?;
    let committed = writer.establish_source_bound(
        &store,
        "retained-replay",
        &candidate,
        store.snapshot().revision(),
        &head,
    )?;
    let graph = graph_reference(&store, 1)?;
    let decisions = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    let [decision] = decisions.as_slice() else {
        return Err("expected one decision".into());
    };
    let pending = ReviewCase::pending(
        ATHLETE_IDENTITY_FAMILY,
        candidate.as_str(),
        "Changed current review",
        "Requires new evidence",
    );
    store.replace(Table::ReviewCases, &pending)?;
    let mut changed_athlete = athlete.clone();
    changed_athlete
        .evidence
        .first_mut()
        .ok_or("original athlete evidence")?
        .observed_on = "2026-10-15".into();
    store.append(Table::Athletes, &changed_athlete)?;
    let mut changed_observation = original
        .observations
        .first()
        .ok_or("original observation")?
        .clone();
    let SourceObservation::Athlete(row) = &mut changed_observation else {
        return Err("fixture source observation is not an athlete".into());
    };
    row.source_row_key = "later-source-row".into();
    store.append(Table::SourceObservations, &changed_observation)?;
    let mut changed_verdict = original.verdicts.first().ok_or("original verdict")?.clone();
    changed_verdict.rationale = "Later retained review evidence".into();
    store.replace(Table::IdentityVerdicts, &changed_verdict)?;
    let current = Inputs::capture(&store.snapshot())?;
    check!(ne; current.athletes, original.athletes);
    check!(ne; current.observations, original.observations);
    check!(ne; current.cases, original.cases);
    check!(ne; current.verdicts, original.verdicts);
    stage_graph(&store, &writer, "changed-live-source-inspection")?;
    let (current_graph, _) = crate::public_identity::graph_operation::inspect(
        &writer,
        &store,
        "changed-live-source-inspection",
    )?;
    let snapshot = store.snapshot();
    let current_view = crate::public_identity::paged_generation::VerifiedRawGraph::verify(
        &snapshot,
        current_graph.capture(),
        current_graph.seal(),
    )?;
    check!(matches!(
        crate::public_identity::authority::graph::accept_source(
            &current_view,
            &candidate,
            &decision.observed_at
        ),
        Err(StoreError::PublicIdentity(
            PublicIdentityError::NotSourceBound
        ))
    ));
    drop(current_view);
    drop(snapshot);
    let snapshot = store.snapshot();
    let view = crate::public_identity::paged_generation::VerifiedRawGraph::verify(
        &snapshot,
        graph.capture(),
        graph.seal(),
    )?;
    let (replayed, _) = crate::public_identity::authority::graph::accept_source(
        &view,
        &candidate,
        &decision.observed_at,
    )?;
    check!(eq; replayed.record(), decision);
    check!(eq; exact_inputs(&graph_inputs(&store, &graph)?)?, exact_inputs(&original)?);
    drop(view);
    drop(snapshot);
    let before = retained_state(&store)?;
    drop(writer);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
            .into_parts();
    check!(eq; writer.head(&store)?, *committed.head());
    let replay = writer.establish_source_bound(
        &store,
        "retained-replay",
        &candidate,
        store.snapshot().revision(),
        &head,
    )?;
    check!(eq; replay.replayed(), true);
    check!(eq; replay.public_id(), committed.public_id());
    check!(eq; replay.head(), committed.head());
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

#[test]
fn abandoned_context_staging_does_not_poison_current_ledger_or_reopen() -> TestResult {
    let (directory, store, writer) = fixture()?;
    install_inputs(&store, &full_inputs()?)?;
    let inputs = Inputs::capture(&store.snapshot())?;
    let candidate = inputs
        .athletes
        .iter()
        .find(|athlete| {
            athlete
                .source
                .as_ref()
                .is_some_and(|source| source.id == "9201")
        })
        .ok_or("first candidate")?
        .id
        .cast();
    let head = writer.head(&store)?;
    let committed = writer.establish_source_bound(
        &store,
        "before-orphan",
        &candidate,
        store.snapshot().revision(),
        &head,
    )?;
    let sequence = store.snapshot().sequence();
    let header = empty_header(writer.store_identity());
    let pass = std::cell::Cell::new(0);
    let payload = "x".repeat(600_000);
    refusal(
        crate::journal::context::stage_context_object(&store, |sink| {
            pass.set(pass.get() + 1);
            sink.record(&wire::Entry {
                key: "header",
                at: "",
                payload: wire::Payload {
                    kind: "header",
                    value: &header,
                },
            })?;
            sink.record(&wire::Entry {
                key: "orphan",
                at: "",
                payload: wire::Payload {
                    kind: "orphan",
                    value: &payload,
                },
            })?;
            if pass.get() == 2 {
                return Err(StoreError::Refused {
                    detail: "synthetic staging interruption".into(),
                });
            }
            Ok(())
        }),
        "synthetic staging interruption",
    )?;
    check!(eq; pass.get(), 2);
    check!(store.snapshot().sequence() > sequence);
    check!(eq; writer.head(&store)?, *committed.head());
    let before = retained_state(&store)?;
    drop(writer);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("identity"))?
            .into_parts();
    check!(eq; writer.head(&store)?, *committed.head());
    check!(eq; retained_state(&store)?, before);
    let replay = writer.establish_source_bound(
        &store,
        "before-orphan",
        &candidate,
        store.snapshot().revision(),
        &head,
    )?;
    check!(eq; replay.replayed(), true);
    check!(eq; replay.public_id(), committed.public_id());
    Ok(())
}

pub(super) fn graph_reference(
    store: &Store,
    sequence: u64,
) -> TestResult<crate::public_identity::graph_reference::GraphAuthorityReferenceV1> {
    let key = crate::public_identity::witness::key(sequence);
    let raw = store.journal.get(&key)?.ok_or("graph witness")?;
    Ok(crate::public_identity::graph_witness::HeldGraphWitness::decode(&key, &raw)?.graph)
}

pub(super) fn graph_inputs(
    store: &Store,
    reference: &crate::public_identity::graph_reference::GraphAuthorityReferenceV1,
) -> TestResult<Inputs> {
    use crate::public_identity::paged_generation::{AuthorityTableV2, VerifiedRawGraph};
    let snapshot = store.snapshot();
    let view = VerifiedRawGraph::verify(&snapshot, reference.capture(), reference.seal())?;
    let mut inputs = Inputs {
        athletes: Vec::new(),
        observations: Vec::new(),
        cases: Vec::new(),
        verdicts: Vec::new(),
    };
    view.for_each_merged(AuthorityTableV2::Athletes, BUDGET, |row, _| {
        inputs.athletes.push(row);
        Ok(())
    })?;
    view.for_each_observation(AuthorityTableV2::SourceObservations, BUDGET, |row, _| {
        inputs.observations.push(row);
        Ok(())
    })?;
    view.for_each_merged(AuthorityTableV2::ReviewCases, BUDGET, |row, _| {
        inputs.cases.push(row);
        Ok(())
    })?;
    view.for_each_merged(AuthorityTableV2::IdentityVerdicts, BUDGET, |row, _| {
        inputs.verdicts.push(row);
        Ok(())
    })?;
    Ok(inputs)
}

pub(super) fn stage_graph(
    store: &crate::Store,
    writer: &crate::PublicIdentityWriterV1,
    operation: &str,
) -> TestResult {
    let before = retained_state(store)?;
    let (graph, _) = crate::public_identity::graph_operation::inspect(writer, store, operation)?;
    let after = retained_state(store)?;
    let raw_operation =
        crate::public_identity::graph_operation::raw_operation(writer.store_identity(), operation)?;
    let reservation_key = format!(
        "manual-context-admission-v1\0reservation\0{}",
        crate::journal::object_digest(raw_operation.as_str().as_bytes())
    );
    let reservation = store
        .journal
        .get(&reservation_key)?
        .ok_or("graph reservation")?;
    let reservation: serde_json::Value = serde_json::from_slice(&reservation)?;
    check!(eq; reservation.get("version").and_then(serde_json::Value::as_u64), Some(3));
    let allowed = staged_graph_keys(&reservation, &reservation_key)?;
    check!(eq; before.len(), after.len());
    for ((before_name, before_rows), (after_name, after_rows)) in before.iter().zip(&after) {
        check!(eq; before_name, after_name);
        if before_name != crate::JOURNAL {
            check!(eq; before_rows, after_rows);
            continue;
        }
        let header_key = b"manual-context-admission-v1\0header";
        for (key, value) in before_rows {
            let (_, current) = after_rows
                .iter()
                .find(|(held, _)| held == key)
                .ok_or("inspection removed a retained row")?;
            if key.as_slice() != header_key {
                check!(eq; current, value);
            }
        }
        let added: Vec<_> = after_rows
            .iter()
            .filter(|(key, _)| !before_rows.iter().any(|(prior, _)| prior == key))
            .collect();
        check!(added.iter().all(|(key, _)| allowed.contains(key)));
        check!(added
            .iter()
            .any(|(key, _)| key == reservation_key.as_bytes()));
        let bytes = added
            .iter()
            .try_fold(0_u64, |total, (key, value)| -> TestResult<u64> {
                let row = key
                    .len()
                    .checked_add(value.len())
                    .ok_or("row charge overflow")?;
                Ok(total
                    .checked_add(u64::try_from(row)?)
                    .ok_or("charge overflow")?)
            })?;
        let roots = u64::try_from(
            added
                .iter()
                .filter(|(key, _)| key.starts_with(b"j2\0manifest\0"))
                .count(),
        )?;
        let header = |rows: &[(Vec<u8>, Vec<u8>)]| -> TestResult<serde_json::Value> {
            let (_, raw) = rows
                .iter()
                .find(|(key, _)| key.as_slice() == header_key)
                .ok_or("admission header")?;
            Ok(serde_json::from_slice(raw)?)
        };
        let mut expected_header = header(before_rows)?;
        for (field, increment) in [("bytes", bytes), ("roots", roots), ("reservations", 1)] {
            let count = expected_header
                .get(field)
                .and_then(serde_json::Value::as_u64)
                .ok_or("admission counter")?;
            *expected_header.get_mut(field).ok_or("admission counter")? =
                serde_json::Value::from(count.checked_add(increment).ok_or("counter overflow")?);
        }
        check!(eq; header(after_rows)?, expected_header);
    }
    let sequence = store.snapshot().sequence();
    let (repeated, _) = crate::public_identity::graph_operation::inspect(writer, store, operation)?;
    check!(eq; repeated, graph);
    check!(eq; retained_state(store)?, after);
    check!(eq; store.snapshot().sequence(), sequence);
    Ok(())
}

fn staged_graph_keys(
    reservation: &serde_json::Value,
    reservation_key: &str,
) -> TestResult<std::collections::BTreeSet<Vec<u8>>> {
    let descriptors = reservation
        .get("descriptors")
        .and_then(serde_json::Value::as_array)
        .ok_or("graph descriptors")?;
    let mut keys = std::collections::BTreeSet::from([reservation_key.as_bytes().to_vec()]);
    for descriptor in descriptors {
        let physical = descriptor.get("physical").ok_or("physical descriptor")?;
        let root = physical
            .get("root")
            .and_then(serde_json::Value::as_str)
            .ok_or("physical root")?;
        keys.insert(crate::journal::object_manifest_key(root)?);
        for (field, chunk) in [("chunks", true), ("pages", false)] {
            let rows = physical
                .get(field)
                .and_then(serde_json::Value::as_array)
                .ok_or("physical rows")?;
            for ordinal in 0..rows.len() {
                keys.insert(if chunk {
                    crate::journal::object_chunk_key(root, ordinal)?
                } else {
                    crate::journal::object_index_key(root, ordinal)?
                });
            }
        }
    }
    Ok(keys)
}

#[test]
fn current_graph_material_fence_ignores_review_metadata_but_binds_retained_member_facts(
) -> TestResult {
    let fixture = super::reviewed_fixture::ready_for_merge()?;
    let members: Vec<_> = fixture.candidates.iter().take(2).cloned().collect();
    let original = current_material(
        &fixture.store,
        &fixture.writer,
        "material-original",
        &members,
    )?;
    let mut inputs = Inputs::capture(&fixture.store.snapshot())?;
    for case in &mut inputs.cases {
        case.subject = "Changed display label".into();
        case.detail = "Another model rerun description".into();
    }
    for verdict in &mut inputs.verdicts {
        verdict.reviewer = "Another synthetic model".into();
        verdict.rationale = "Same facts, new wording".into();
    }
    fixture
        .store
        .replace_many(Table::ReviewCases, &inputs.cases)?;
    fixture
        .store
        .replace_many(Table::IdentityVerdicts, &inputs.verdicts)?;
    check!(eq; current_material(&fixture.store, &fixture.writer, "material-metadata", &members)?, original);
    let unrelated_id = fixture.candidates.last().ok_or("unrelated candidate")?;
    let unrelated = inputs
        .athletes
        .iter_mut()
        .find(|row| row.id.as_str() == unrelated_id.as_str())
        .ok_or("unrelated athlete")?;
    unrelated
        .evidence
        .push(census_domain::model::Evidence::parsed(
            census_domain::model::SourceRef::new(
                "unrelated later source",
                Some("https://unrelated-source.test/result".into()),
            ),
            "2026-10-15",
        ));
    fixture.store.append(Table::Athletes, unrelated)?;
    check!(eq; current_material(&fixture.store, &fixture.writer, "material-unrelated", &members)?, original);
    let member_id = members.first().ok_or("member")?;
    let member = inputs
        .athletes
        .iter_mut()
        .find(|row| row.id.as_str() == member_id.as_str())
        .ok_or("member athlete")?;
    member.evidence.push(census_domain::model::Evidence::parsed(
        census_domain::model::SourceRef::new(
            "member later source",
            Some("https://member-source.test/result".into()),
        ),
        "2026-10-15",
    ));
    fixture.store.append(Table::Athletes, member)?;
    check!(ne; current_material(&fixture.store, &fixture.writer, "material-member", &members)?, original);
    Ok(())
}

fn current_material(
    store: &Store,
    writer: &crate::PublicIdentityWriterV1,
    operation: &str,
    members: &[census_domain::model::AthleteCandidateId],
) -> TestResult<String> {
    stage_graph(store, writer, operation)?;
    let (reference, _) =
        crate::public_identity::graph_operation::inspect(writer, store, operation)?;
    let before = retained_state(store)?;
    let snapshot = store.snapshot();
    let view = crate::public_identity::paged_generation::VerifiedRawGraph::verify(
        &snapshot,
        reference.capture(),
        reference.seal(),
    )?;
    let value = crate::public_identity::authority::graph::identity_material(&view, members)?;
    check!(eq; retained_state(store)?, before);
    Ok(value)
}
