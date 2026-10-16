use super::*;
use context::{Context, History, Stage};

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
    for (command, receipt) in completed {
        let before = preservation::Held::capture(&store, &sources)?;
        let replay = store.prepare_evidence_subject_attachment_v1(
            command.clone(),
            EvidenceSubjectAttachmentPreparationV1::ReplayOnly,
        )?;
        check!(replay.is_replay());
        let approval = replay.approval_bytes().to_vec();
        let digest = replay.approval_digest().clone();
        match store.apply_evidence_subject_attachment_v1(replay, &approval, &digest)? {
            Application::Repeated(found) => check!(eq; found, receipt),
            Application::Written(_) => {
                return Err("exact replay unexpectedly wrote another attachment".into())
            }
        }
        before.require_unchanged(&store, &sources)?;
        report::phase(
            command.operation.as_str(),
            "replay",
            "repeated-exact-receipt",
            json!(receipt),
        )?;
    }
    drop(writer);
    Ok(())
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
