use crate::public_identity::{
    authority, context,
    graph_witness::{HeldGraphWitness, Payload},
    PublicIdentityError,
};
use crate::public_identity_tests::support::{fixture, retained_state, subject, TestResult};
use crate::{StoreError, Table};
use census_domain::model::AppliedIdentityKind;
use census_domain::public_identity_history::{
    CandidateDecisionReference, PublicAthleteId, ReplayLimits,
};

mod codec;
mod graph_fixture;
mod graph_operations;
mod refusal;

fn inputs(value: &str) -> TestResult<context::Inputs> {
    let athletes = vec![subject("reviewed-a", "41")?, subject("reviewed-b", "42")?];
    let members = athletes
        .iter()
        .map(|athlete| athlete.id.cast())
        .collect::<Vec<_>>();
    let mut inputs = context::Inputs {
        athletes,
        observations: Vec::new(),
        cases: Vec::new(),
        verdicts: Vec::new(),
    };
    let (case, verdict) = context::reviewed_fixture::review_inputs(&inputs, &members, value)?;
    inputs.cases.push(case);
    inputs.verdicts.push(verdict);
    Ok(inputs)
}

fn case_id(inputs: &context::Inputs) -> TestResult<&str> {
    Ok(&inputs.cases.first().ok_or("review case")?.id)
}

fn public_id() -> TestResult<PublicAthleteId> {
    Ok(PublicAthleteId::parse(&format!(
        "ath_public_v1_{}",
        "0".repeat(64)
    ))?)
}

#[test]
fn reviewed_authority_roundtrips_real_different_person_context() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let inputs = inputs("different_person")?;
    let original = serde_json::to_value((
        &inputs.athletes,
        &inputs.observations,
        &inputs.cases,
        &inputs.verdicts,
    ))?;
    let accepted = authority::accept_reviewed(&inputs, case_id(&inputs)?, "2026-10-14")?;
    check!(eq; accepted.record().kind, AppliedIdentityKind::DifferentPerson);
    let evidence = CandidateDecisionReference::from_correction(&accepted, ReplayLimits::SMALL)?;
    check!(evidence.matches_correction(accepted.record(), ReplayLimits::SMALL)?);
    let staged = inputs.stage(&store, writer.store_identity())?;
    let before = retained_state(&store)?;
    context::verify_reviewed(
        &store.db.snapshot(),
        &store.journal,
        staged.object_id(),
        accepted.record(),
    )?;
    check!(eq; serde_json::to_value((&inputs.athletes, &inputs.observations, &inputs.cases, &inputs.verdicts))?, original);
    check!(eq; retained_state(&store)?, before);
    check!(eq; writer.head(&store)?.sequence, 0);
    let mut changed = accepted.record().clone();
    changed.verdict_digest = Some("0".repeat(64));
    let refused = context::verify_reviewed(
        &store.db.snapshot(),
        &store.journal,
        staged.object_id(),
        &changed,
    );
    check!(matches!(refused, Err(StoreError::Refused { detail })
        if detail == "retained reviewed authority decision does not replay"));
    check!(eq; retained_state(&store)?, before);
    Ok(())
}

fn stage_graph(
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
