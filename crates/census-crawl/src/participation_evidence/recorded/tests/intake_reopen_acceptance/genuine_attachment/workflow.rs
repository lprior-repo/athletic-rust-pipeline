use super::*;
use context::{Context, History, Stage};

mod discovery_check;
mod handoff;
mod operations;

use operations::{chain, require_originals};

#[test]
fn genuine_attachment_complete_inventory_applies_a_then_b_and_reopens_or_reports_exact_blocker(
) -> TestResult {
    let (directory, store, writer, installed) =
        super::super::accepted_provider::installed::install()?;
    let outcome = writer.establish_source_bound(
        &store,
        "genuine-attachment-source-bound-target",
        &installed.candidate,
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    check!(!outcome.replayed());
    check!(eq; outcome.head().sequence, 1);
    installed.verify(&store)?;
    let sources = Sources::install_and_read(&store)?;
    require_originals(&store, &sources)?;
    let source_tables = store.snapshot().tables_digest(&Table::ALL)?;
    let generations = (
        store.evidence_generation(),
        store.snapshot().derived_generation(),
    );
    let result = chain(&store, &sources, &outcome);
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, source_tables);
    check!(eq; (store.evidence_generation(), store.snapshot().derived_generation()), generations);
    require_originals(&store, &sources)?;
    let retained = preservation::Logical::capture(&store, &sources)?;
    drop(writer);
    drop(store);
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("store"))?.into_parts();
    check!(eq; preservation::Logical::capture(&store, &sources)?, retained);
    retained.require_receipts(&store)?;
    require_originals(&store, &sources)?;
    installed.verify(&store)?;
    report::phase(
        "chain",
        "reopen",
        "preserved",
        report::physical_inventory(&store, &sources)?,
    )?;
    let completed = match result {
        Ok(completed) => completed,
        Err(error) => {
            report::phase(
                "chain",
                "acceptance",
                "blocked",
                json!({"error":error.to_string(),
                "replay":"Not claimed for an incomplete chain", "b":"Requires actual successful active A; no replacement target or reduced inventory used"}),
            )?;
            return Err(error);
        }
    };
    if let Err(error) = replay_completed(&store, &sources, completed)
        .and_then(|()| super::bridge::verify(&store, &sources, &installed, &outcome))
        .and_then(|()| {
            eprintln!("genuine_driver_stage discovery-start");
            discovery_check::verify(&store, &sources)
        })
        .and_then(|()| {
            eprintln!("genuine_driver_stage attached-participation-start");
            crate::participation_evidence::recorded::verify_genuine_attached_participation(&store)
        })
        .and_then(|()| {
            eprintln!("genuine_driver_stage adversarial-start");
            crate::participation_evidence::recorded::verify_adversarial_attached_participation(
                &store,
            )
        })
    {
        drop(writer);
        drop(store);
        let observed = (|| -> TestResult {
            let (store, _writer) =
                Store::open_public_identity_qualification_v3(directory.path().join("store"))?
                    .into_parts();
            check!(eq; preservation::Logical::capture(&store, &sources)?, retained);
            retained.require_receipts(&store)?;
            require_originals(&store, &sources)?;
            installed.verify(&store)
        })();
        replay_observation("chain", "replay-or-bridge-failure-reopen", observed);
        return Err(error);
    }
    let prepared = handoff::prepare(&store, &sources);
    drop(writer);
    drop(store);
    eprintln!("genuine_driver_stage targets-start");
    if let Err(error) =
        crate::participation_evidence::recorded::verify_targets_attached_participation(
            &directory.path().join("store"),
        )
        .and_then(|()| {
            eprintln!("genuine_driver_stage source-count-start");
            crate::participation_evidence::recorded::verify_source_count_attached_participation(
                &directory.path().join("store"),
            )
        })
    {
        let _preserved = directory.keep();
        return Err(error);
    }
    match prepared {
        Ok(Some(prepared)) => handoff::finish(prepared, directory),
        Ok(None) => Ok(()),
        Err(error) => {
            let _preserved = directory.keep();
            Err(error)
        }
    }
}

fn replay_completed(
    store: &Store,
    sources: &Sources,
    completed: Vec<(RetainedEvidenceSubjectAttachmentCommandV1, Receipt)>,
) -> TestResult {
    for (command, receipt) in completed {
        let before = preservation::Held::capture(store, sources)?;
        replay_phase(
            command.operation.as_str(),
            "replay-prepare-start",
            "started",
            json!({}),
        );
        let replay = match store.prepare_evidence_subject_attachment_v1(
            command.clone(),
            EvidenceSubjectAttachmentPreparationV1::ReplayOnly,
        ) {
            Ok(replay) => replay,
            Err(error) => {
                return replay_refused(
                    command.operation.as_str(),
                    "replay-prepare",
                    error,
                    store,
                    sources,
                    &before,
                )
            }
        };
        check!(replay.is_replay());
        replay_phase(
            command.operation.as_str(),
            "replay-prepare",
            "prepared",
            json!({}),
        );
        let approval = replay.approval_bytes().to_vec();
        let digest = replay.approval_digest().clone();
        replay_phase(
            command.operation.as_str(),
            "replay-apply-start",
            "started",
            json!({}),
        );
        match store.apply_evidence_subject_attachment_v1(replay, &approval, &digest) {
            Ok(Application::Repeated(found)) => {
                check!(eq; found, receipt);
                replay_phase(
                    command.operation.as_str(),
                    "replay-apply",
                    "repeated-exact-receipt",
                    json!(found),
                );
            }
            Ok(Application::Written(_)) => {
                return Err("exact replay unexpectedly wrote another attachment".into())
            }
            Err(error) => {
                return replay_refused(
                    command.operation.as_str(),
                    "replay-apply",
                    error,
                    store,
                    sources,
                    &before,
                )
            }
        }
        before.require_unchanged(store, sources)?;
        report::phase(
            command.operation.as_str(),
            "replay",
            "repeated-exact-receipt",
            json!(receipt),
        )?;
    }
    Ok(())
}

fn replay_refused(
    stage: &str,
    phase: &str,
    error: StoreError,
    store: &Store,
    sources: &Sources,
    before: &preservation::Held<'_>,
) -> TestResult {
    let detail = match &error {
        StoreError::EvidenceSubjectAttachment(EvidenceSubjectAttachmentStoreError::Domain(
            EvidenceSubjectAttachmentError::Capacity {
                resource,
                needed,
                available,
            },
        )) => json!({"typed":"EvidenceSubjectAttachment::Domain::Capacity",
            "resource":resource, "needed":needed, "available":available}),
        other => {
            json!({"typed":"StoreError", "category":format!("{:?}", std::mem::discriminant(other)),
            "detail":"Original typed error returned unchanged; no source payload emitted"})
        }
    };
    replay_phase(stage, phase, "refused", detail);
    let observed = before
        .require_unchanged(store, sources)
        .and_then(|()| require_originals(store, sources));
    replay_observation(stage, &format!("{phase}-preservation"), observed);
    Err(error.into())
}

fn replay_observation(stage: &str, phase: &str, observed: TestResult) {
    let outcome = if observed.is_ok() {
        "preserved"
    } else {
        "observation-failed"
    };
    replay_phase(
        stage,
        phase,
        outcome,
        json!({
            "checks":"Exact namespace/logical payloads, receipts and original roots; reopened observation also checks accepted-provider identity",
            "primary_error":"Original replay failure remains the returned error",
            "observer_error":"Any observer failure is reported separately without source payloads",
        }),
    );
}

fn replay_phase(stage: &str, phase: &str, outcome: &str, detail: Value) {
    if report::phase(stage, phase, outcome, detail).is_err() {
        eprintln!("GENUINE_ATTACHMENT replay-diagnostic-write-failed stage={stage} phase={phase} outcome={outcome}");
    }
}

#[test]
fn genuine_attachment_literal_fact_inventory_keeps_article_both_chsl_and_all_three_may15_rows(
) -> TestResult {
    let mut objects = Objects::default();
    for (body, offset, length, digest) in [
        (
            STATUS,
            113940,
            588,
            "d668ec9ee0510d228cb19e0bba2e08fe934eccfc80548280f3ec56329e1d0428",
        ),
        (
            RESULTS,
            65480,
            1319,
            "93fa4b8db2c428b36aacd581f89720d3dc49533e400064803178cb4f05f1453d",
        ),
        (
            RESULTS,
            69033,
            1319,
            "2a6e216d180730bc6724860d98a71312803f4a8473ec2ec75f444161257de2a4",
        ),
        (
            HTML,
            80520,
            100,
            "bc7c7f83bcfc715c9c3343e09cf45f82bee7651ac86838f3427e1097de010b3d",
        ),
        (
            HTML,
            81648,
            92,
            "6bf39ca146977ced76ba63e226c1d41786e1914947f492c148fe3ca8708cd582",
        ),
        (
            HTML,
            84465,
            92,
            "8595ae83c23900e31b747b8bc0b3c6adb7c842139e0344f15e361a71e718d736",
        ),
    ] {
        digest_span(&mut objects, body, offset, length, digest)?;
    }
    check!(eq; objects.0.len(), 6);
    Ok(())
}
