use super::{codec, ledger, prefix, transport, TestResult};
use crate::restate_services::wire::{JurisdictionRequest, NationalRequest};
use crate::restate_services::CaptureExecution;
use census_domain::model::{CensusRun, GradYear, RunManifest};
use census_domain::UsJurisdiction;
use census_reconcile::identity::WorkflowIdentity;
use census_store::{Store, Table};
use std::sync::atomic::Ordering;
use std::sync::Arc;

fn national_prefix() -> TestResult<prefix::Journal> {
    prefix::read_journal(prefix::CaptureSpec {
        bytes: include_bytes!("national_20261021.journal.json"),
        digest: "1c98e3391ad2cea6e60046bcd4ebb09c36b0e1faa4903ab95af00e5541ea88c0",
        observed_at: "2026-10-21T14:54:36.297Z",
        key: "national:2026-27:bfaa5c60f1fd9fbd:1",
        invocation: "inv_14jQCQ4alEcj3X8qiRl6rijMlf95fHjXxP",
        entries: 45,
    })
}

fn illinois_prefix() -> TestResult<prefix::Journal> {
    prefix::read_journal(prefix::CaptureSpec {
        bytes: include_bytes!("illinois_pass_20261021.journal.json"),
        digest: "d66b119f01e81711ca00d64b6b7b15b0e1248264ca0ed90178513764c401ce6e",
        observed_at: "2026-10-21T14:54:36.296Z",
        key: "jurisdiction:IL:2026-27:1",
        invocation: "inv_13ABzPGQVhe85vi68Uoaj99IngyCZY99PH",
        entries: 10,
    })
}

#[tokio::test]
async fn pre_change_illinois_pass_journal_replays_to_the_same_pending_ihsa_call_without_effects(
) -> TestResult {
    let prefix = illinois_prefix()?;
    let request: JurisdictionRequest = prefix.input()?;
    check!(eq; request.jurisdiction, UsJurisdiction::Illinois);
    check!(eq; WorkflowIdentity::jurisdiction(request.jurisdiction, request.season, request.revision).as_str(), prefix.key);
    let retained_state = prefix.frames.get(1).ok_or("missing original state read")?;
    check!(eq; retained_state.kind, 0x0407);
    let last_call = prefix.frames.get(8).ok_or("missing original IHSA Call")?;
    check!(eq; last_call.text(1)?, "TeamsSource");
    check!(eq; last_call.text(2)?, "run");
    check!(eq; last_call.text(5)?, "jurisdiction:IL:2026-27:1/teams/ihsa");
    check!(eq; last_call.number(11)?, 6);
    replay(prefix, "JurisdictionCensus/pass", 6, false).await
}

#[tokio::test]
async fn pre_change_national_run_journal_replays_to_the_same_pending_illinois_pass_without_effects(
) -> TestResult {
    let prefix = national_prefix()?;
    let request: NationalRequest = prefix.input()?;
    check!(eq; WorkflowIdentity::national(request.season, request.revision, &request.jurisdictions).as_str(), prefix.key);
    let selection = request.export_job.require_selected_publication();
    check!(eq; selection.store_identity().as_str(), "fea26b14cb1bcac9807665773ae3db5559ddfe7cfea59cb457956f807bc59879");
    check!(eq; selection.profile_id().as_str(), "29aa9c73788a18bf94dad7d313ec58f0dd12da17f612aa20001f7466fbd3b940");
    let last_call = prefix
        .frames
        .get(39)
        .ok_or("missing original Illinois pass Call")?;
    check!(eq; last_call.text(1)?, "JurisdictionCensus");
    check!(eq; last_call.text(2)?, "pass");
    check!(eq; last_call.text(5)?, "jurisdiction:IL:2026-27:1");
    check!(eq; last_call.number(11)?, 25);
    replay(prefix, "NationalCensus/run", 25, true).await
}

async fn replay(
    prefix: prefix::Journal,
    handler: &str,
    pending: u8,
    unordered: bool,
) -> TestResult {
    let original: NationalRequest = national_prefix()?.input()?;
    let root = tempfile::tempdir()?;
    Store::create_current_qualification_v3(root.path())?;
    let (store, writer) = Store::open_current_qualification_v3(root.path())?.into_parts();
    let store = Arc::new(store);
    let manifest = RunManifest {
        store_identity: store.store_identity()?.as_str().to_owned(),
        run: CensusRun::new(original.season, original.revision.get())
            .ok_or("retained run invalid")?,
        cohort: GradYear::CO2027,
        jurisdictions: original.jurisdictions.clone(),
    };
    check!(eq; manifest.store_identity, writer.store_identity().as_str());
    check!(
        manifest.store_identity
            != original
                .export_job
                .require_selected_publication()
                .store_identity()
                .as_str()
    );
    store.bind_run(&manifest)?;
    seed_recorded_status(&store, &prefix, handler)?;
    let before = protected(&store)?;
    let clock = Arc::new(transport::UnusedClock::default());
    let frames = transport::exchange_recorded(
        &store,
        &clock,
        CaptureExecution::FreshQualification(Arc::new(writer)),
        &prefix,
        handler,
    )
    .await?;
    assert_pending(&frames, pending, unordered)?;
    check!(eq; clock.0.load(Ordering::SeqCst), 0);
    check!(eq; protected(&store)?, before);
    check!(eq; store.run_manifest()?, Some(manifest));
    check!(store.journal_keys(ledger::PHASE)?.is_empty());
    check!(store
        .journal_keys(ledger::origin_continuation::PHASE)?
        .is_empty());
    let attempts = ledger::durable_attempts(&store)?;
    check!(eq; (attempts.sources, attempts.reserved, attempts.recorded), (0, 0, 0));
    Ok(())
}

fn seed_recorded_status(store: &Store, prefix: &prefix::Journal, handler: &str) -> TestResult {
    if handler != "JurisdictionCensus/pass" {
        return Ok(());
    }
    let state = prefix.frames.get(1).ok_or("missing retained state")?;
    check!(eq; state.kind, 0x0407);
    let value = codec::field(state.bytes(14)?, 1)?.ok_or("missing retained state value")?;
    let state: crate::restate_services::wire::JurisdictionState = serde_json::from_slice(value)?;
    let request: JurisdictionRequest = prefix.input()?;
    let fingerprint = crate::restate_services::plan::compute_plan_fingerprint(
        request.jurisdiction,
        request.season,
        request.revision,
        crate::restate_services::plan::BrowserLaneState::Configured,
    );
    check!(eq; state.plan.as_ref().ok_or("retained plan missing")?.fingerprint, fingerprint);
    let status = state
        .athleticlive
        .ok_or("retained AthleticLIVE status missing")?;
    let key = format!("{}/athleticlive_manifest/status", prefix.key);
    store.journal_done("athleticlive_manifest_obligation_v1", &key, &status)?;
    Ok(())
}

fn protected(store: &Store) -> TestResult<(u64, u64, u64, String)> {
    let snapshot = store.snapshot();
    Ok((
        snapshot.sequence(),
        snapshot.evidence_generation(),
        snapshot.derived_generation(),
        snapshot.tables_digest(&Table::ALL)?,
    ))
}

fn assert_pending(frames: &[codec::Frame], pending: u8, unordered: bool) -> TestResult {
    let mut completion = codec::blob(1, &[pending])?;
    let mut expected = if unordered {
        completion.extend(codec::integer(5, 1));
        let mut future = codec::blob(2, &[1])?;
        future.extend(codec::blob(4, &completion)?);
        future
    } else {
        completion.extend(codec::blob(2, &[1])?);
        completion
    };
    expected.extend(codec::integer(5, 1));
    check!(
        (1..=2).contains(&frames.len()),
        "unexpected SDK output: {frames:?}"
    );
    let (suspension, updates) = frames
        .split_last()
        .ok_or("missing original Call suspension")?;
    check!(eq; suspension.kind, 0x0001, "SDK output {frames:?}");
    check!(eq; suspension.payload, codec::blob(4, &expected)?);
    for update in updates {
        check!(eq; update.kind, 0x0006);
        check!(eq; update.payload, codec::blob(1, &expected)?);
    }
    Ok(())
}
