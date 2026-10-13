use super::*;
use crate::restate_services::wire::{HistoryWindow, SourcePlan};
use census_domain::{
    model::{normalize_name, CanonicalMeet, CanonicalPerformance, CanonicalSchool, SchoolYear},
    UsJurisdiction,
};
use census_reconcile::identity::Revision;
use census_store::Table;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

mod fixtures;
use fixtures::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn athleticlive_all_twelve_missing_manifests_persist_exact_refusals_and_rebind_once() -> TestResult
{
    let root = tempfile::tempdir()?;
    let jurisdictions = census_crawl::applicability::table()
        .iter()
        .find(|row| row.slug == "athleticlive")
        .ok_or("applicability")?
        .jurisdictions;
    check!(eq; jurisdictions.len(), 12);
    {
        let store = Store::open(root.path())?;
        for jurisdiction in jurisdictions {
            let request = request(*jurisdiction)?;
            let first = ledger::bind(&store, &request)?.ok_or("obligation")?;
            let again = ledger::bind(&store, &request)?.ok_or("rebind")?;
            check!(eq; serde_json::to_value(&first)?, serde_json::to_value(again)?);
            let AthleticLiveObligation::Refused { reason } = first else {
                return Err("missing manifest armed".into());
            };
            check!(eq; reason, format!("{}: no qualified retained AthleticLIVE manifest is registered for this jurisdiction run; operator captures with physical producer metadata are required; no live discovery or invented objects", jurisdiction.code()));
            let state = state(&store, &request)?;
            let (owed, reasons) = super::super::owed_work(&state, &request);
            check!(owed.iter().any(|value| value == "athleticlive_manifest"));
            check!(reasons.contains(&format!("athleticlive_manifest: refused: {reason}")));
        }
        check!(eq; store.journal_keys(ledger::PHASE)?.len(), 12);
        store.flush()?;
    }
    let store = Store::open(root.path())?;
    for jurisdiction in jurisdictions {
        let request = request(*jurisdiction)?;
        let status = ledger::status(&store, &request)?.ok_or("persisted refusal")?;
        check!(
            matches!(status, AthleticLiveObligation::Refused { reason } if reason == ledger::missing_reason(&request))
        );
    }
    check!(eq; store.journal_keys(ledger::PHASE)?.len(), 12);
    check!(ledger::bind(&store, &request(UsJurisdiction::California)?)?.is_none());
    Ok(())
}

#[test]
fn athleticlive_fixture_is_armed_archived_executed_and_reopened_without_duplicate_effects(
) -> TestResult {
    let root = tempfile::tempdir()?;
    let source = tempfile::tempdir()?;
    let request = request(UsJurisdiction::Iowa)?;
    let input = manifest(&source)?;
    {
        let store = Store::open(root.path())?;
        seed_schools(&store)?;
        let first = register(&store, &request, &input, "2026-10-12")?;
        check!(matches!(first, AthleticLiveObligation::Armed { .. }));
        let retained = ledger::manifest(&store, &request)?;
        let archived: Value = serde_json::from_str(&retained.body)?;
        let capture = archived
            .pointer("/meets/0/documents/0")
            .and_then(Value::as_str)
            .ok_or("frozen capture path")?;
        check!(eq; std::fs::read(capture)?, XC.as_bytes());
        let archive = store.http_cache_dir().join("archive/bodies");
        let captures = std::fs::read_dir(&archive)?.collect::<Result<Vec<_>, _>>()?;
        check!(eq; captures.len(), 1);
        let before = store.journal_keys(ledger::PHASE)?;
        let rebound = register(&store, &request, &input, "2026-10-13")?;
        check!(eq; serde_json::to_value(first)?, serde_json::to_value(rebound)?);
        check!(eq; store.journal_keys(ledger::PHASE)?, before);
        let rebound_captures = std::fs::read_dir(&archive)?.collect::<Result<Vec<_>, _>>()?;
        check!(eq; rebound_captures.len(), 1);
        check!(eq;
            serde_json::to_value(ledger::manifest(&store, &request)?)?,
            serde_json::to_value(retained)?
        );
        let state = state(&store, &request)?;
        check!(super::super::owed_work(&state, &request)
            .0
            .iter()
            .any(|value| value == "athleticlive_manifest"));
        store.flush()?;
    }
    source.close()?;
    {
        let store = Store::open(root.path())?;
        let result = execute(&store, &request, "2026-10-12")?.ok_or("executed obligation")?;
        let AthleticLiveObligation::Finished {
            attempt, outcome, ..
        } = result
        else {
            return Err("collector did not finish".into());
        };
        check!(eq; attempt, 1);
        check!(eq; outcome.records, 136);
        check!(eq; outcome.disposition, CollectionDisposition::Partial);
        check!(outcome.errors.is_empty());
        let summary = "https://s-gke-usc1-nssi3-33.firebaseio.com/meet_58504/event_summary.json?ns=trackmeet-io";
        check!(eq; outcome.unfinished, vec![summary]);
        check!(outcome
            .notes
            .iter()
            .any(|note| note.contains("requests: 0 by construction")));
        check!(eq; store.scan::<CanonicalMeet>(Table::Meets)?.len(), 1);
        let rows = store.scan::<CanonicalPerformance>(Table::Performances)?;
        check!(eq; rows.len(), 136);
        check!(rows
            .iter()
            .all(|row| row.source_key.starts_with("athleticlive:2150205:")));
        check!(eq;
            store.journal_keys("athleticlive_results_capture_v1")?.len(),
            1
        );
        check!(eq;
            store.journal_keys("athleticlive_results_effect_v3")?.len(),
            1
        );
        let before = store.journal_keys(ledger::PHASE)?;
        let state = state(&store, &request)?;
        let (owed, reasons) = super::super::owed_work(&state, &request);
        check!(owed.iter().any(|name| name == "athleticlive_manifest"));
        check!(reasons
            .iter()
            .any(|reason| reason.contains("records=136") && reason.contains(summary)));
        let replay = execute(&store, &request, "2026-10-13")?.ok_or("replayed completion")?;
        check!(!replay.is_terminal());
        check!(eq;
            store
                .scan::<CanonicalPerformance>(Table::Performances)?
                .len(),
            136
        );
        check!(eq;
            store.journal_keys("athleticlive_results_effect_v3")?.len(),
            1
        );
        check!(eq; before.len(), 4);
        check!(eq; store.journal_keys(ledger::PHASE)?.len(), 6);
        let first_receipt = store
            .journal_payload(
                ledger::PHASE,
                &format!("{}/attempt/1/finish", ledger::key(&request)),
            )?
            .ok_or("first durable receipt")?;
        check!(eq; first_receipt.pointer("/outcome/records"), Some(&json!(136)));
        store.flush()?;
    }
    let store = Store::open(root.path())?;
    check!(eq;
        store
            .scan::<CanonicalPerformance>(Table::Performances)?
            .len(),
        136
    );
    check!(matches!(
        ledger::status(&store, &request)?,
        Some(AthleticLiveObligation::Finished { attempt: 2, outcome, .. }) if outcome.records == 0 && outcome.disposition == CollectionDisposition::Partial
    ));
    let receipt = store
        .journal_payload(
            ledger::PHASE,
            &format!("{}/attempt/1/finish", ledger::key(&request)),
        )?
        .ok_or("reopened first durable receipt")?;
    check!(eq; receipt.pointer("/outcome/records"), Some(&json!(136)));
    Ok(())
}

#[test]
fn athleticlive_malformed_capture_is_retained_and_does_not_hide_a_later_object() -> TestResult {
    let root = tempfile::tempdir()?;
    let source = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    seed_schools(&store)?;
    let request = request(UsJurisdiction::Iowa)?;
    let input = manifest(&source)?;
    let mut envelope: Value = serde_json::from_str(&std::fs::read_to_string(&input)?)?;
    let broken_path = source
        .path()
        .join("malformed.json")
        .to_string_lossy()
        .into_owned();
    let broken = "{not JSON";
    std::fs::write(&broken_path, broken)?;
    let entry = envelope.pointer_mut("/meets/0").ok_or("entry")?;
    entry["documents"]
        .as_array_mut()
        .ok_or("documents")?
        .insert(0, json!(broken_path));
    entry["captures"][&broken_path] = json!({"url":"https://athleticlive.blob.core.windows.net/$web/ind_res_list/_doc/9999999", "method":"GET", "status":200, "content_digest":format!("{:x}", Sha256::digest(broken.as_bytes())), "bytes":broken.len(), "fetched_at":"2026-09-22T03:59:19Z"});
    std::fs::write(&input, envelope.to_string())?;
    register(&store, &request, &input, "2026-10-12")?;
    let outcome = execute(&store, &request, "2026-10-12")?.ok_or("progress")?;
    check!(!outcome.is_terminal());
    let AthleticLiveObligation::Finished { outcome, .. } = outcome else {
        return Err("no finish receipt".into());
    };
    check!(eq; outcome.records, 136);
    check!(!outcome.errors.is_empty());
    check!(eq; outcome.disposition, CollectionDisposition::Partial);
    check!(!outcome.unfinished.is_empty());
    check!(eq;
        store
            .scan::<CanonicalPerformance>(Table::Performances)?
            .len(),
        136
    );
    let state = state(&store, &request)?;
    let (owed, reasons) = super::super::owed_work(&state, &request);
    check!(owed.iter().any(|stage| stage == "athleticlive_manifest"));
    check!(reasons
        .iter()
        .any(|reason| reason.contains("records=136") && reason.contains("unfinished=")));
    check!(eq;
        store.journal_keys("athleticlive_results_capture_v1")?.len(),
        2
    );
    Ok(())
}

#[test]
fn athleticlive_additive_state_decodes_old_bytes_and_round_trips_new_bytes() -> TestResult {
    let old = json!({"identity":"historical", "teams":{"status":"owed"}});
    let state: JurisdictionState = serde_json::from_value(old)?;
    check!(state.athleticlive.is_none());
    let mut state = state;
    state.athleticlive = Some(AthleticLiveObligation::Armed {
        digest: "d9a828d6ca14a531e93044b6f49ebcf84476e544c5d046cfc78c4a6521eb31e7".into(),
    });
    let encoded = serde_json::to_value(&state)?;
    let decoded: JurisdictionState = serde_json::from_value(encoded.clone())?;
    check!(eq; serde_json::to_value(decoded)?, encoded);
    #[derive(serde::Deserialize)]
    struct PreviousState {
        identity: String,
    }
    let previous: PreviousState = serde_json::from_value(encoded)?;
    check!(eq; previous.identity, "historical");
    Ok(())
}

#[test]
fn athleticlive_manifest_without_captured_objects_is_a_persisted_exact_refusal() -> TestResult {
    let root = tempfile::tempdir()?;
    let source = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let request = request(UsJurisdiction::Iowa)?;
    let input = source.path().join("empty-owner.json");
    std::fs::write(
        &input,
        r#"{"meets":[{"athleticlive_meet_id":58504,"tenant":"live_results","name":"Iowa High School State Championships","state":"IA","date":"2025-10-31"}]}"#,
    )?;
    let outcome = register(
        &store,
        &request,
        input.to_str().ok_or("path")?,
        "2026-10-12",
    )?;
    let expected = "IA: retained AthleticLIVE manifest cannot qualify: manifest meet 58504 has no retained summary, event document or standings capture";
    check!(matches!(outcome, AthleticLiveObligation::Refused { reason } if reason == expected));
    check!(
        matches!(ledger::status(&store, &request)?, Some(AthleticLiveObligation::Refused { reason }) if reason == expected)
    );
    check!(eq; store.journal_keys(ledger::PHASE)?.len(), 1);
    Ok(())
}

#[test]
fn athleticlive_rebind_cannot_change_the_immutable_history_window() -> TestResult {
    let root = tempfile::tempdir()?;
    let source = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let mut request = request(UsJurisdiction::Iowa)?;
    let input = manifest(&source)?;
    register(&store, &request, &input, "2026-10-12")?;
    request.history = HistoryWindow::new(2024, 2026, "2026-10-12")?;
    let result = register(&store, &request, &input, "2026-10-12");
    check!(
        matches!(result, Err(JobError::Terminal { message }) if message == "AthleticLIVE manifest history scope changed within logical obligation")
    );
    Ok(())
}
