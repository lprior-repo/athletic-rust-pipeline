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
    check!(eq; witness_root(&store, 1)?, staged.object_id());
    check!(eq; witness_root(&store, 2)?, staged.object_id());
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
    let root = witness_root(&store, 1)?;
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
    match accept_inputs(&current, &candidate, &decision.observed_at) {
        Err(StoreError::PublicIdentity(PublicIdentityError::NotSourceBound)) => {}
        Err(error) => return Err(format!("unexpected current authority error: {error:?}").into()),
        Ok(_) => return Err("current unresolved review accepted".into()),
    }
    read::verify(&store.db.snapshot(), &store.journal, &root, decision)?;
    check!(eq; exact_inputs(&read_inputs(&store, &root)?)?, exact_inputs(&original)?);
    let before = retained_state(&store)?;
    drop(writer);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v2(directory.path().join("identity"))?
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
        Store::open_public_identity_qualification_v2(directory.path().join("identity"))?
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
