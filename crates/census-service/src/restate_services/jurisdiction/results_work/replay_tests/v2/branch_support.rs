use super::*;
use crate::restate_services::national::publication_tests::codec;
use crate::restate_services::wire::JurisdictionState;
use census_domain::model::{serialized_bytes, serialized_digest};
use std::path::PathBuf;

pub(super) struct TerminalTruth {
    pub(super) terminal: ImmutableAttemptResult,
    pub(super) receipt: Receipt,
    terminal_bytes: Vec<u8>,
    receipt_bytes: Vec<u8>,
}

pub(super) fn truth(
    fixture: &Fixture,
    prepared: &fixture::Prepared,
    ordinal: u8,
) -> TestResult<TerminalTruth> {
    let key = format!(
        "work/{}/attempt/{ordinal}/terminal",
        prepared.attempt.work_id()
    );
    let terminal: ImmutableAttemptResult = serde_json::from_value(
        fixture
            .store
            .journal_payload(PHASE, &key)?
            .ok_or("terminal absent")?,
    )?;
    check!(eq; terminal.attempt().work_id(), prepared.attempt.work_id());
    check!(eq; terminal.attempt().ordinal().get(), ordinal);
    let receipt = fixture
        .store
        .receipt(&format!("{PHASE}/{key}"))?
        .ok_or("receipt absent")?;
    check!(eq; receipt.digest, serialized_digest(&terminal)?);
    check!(eq; receipt.appended, 0);
    check!(!receipt.credited);
    Ok(TerminalTruth {
        terminal_bytes: serialized_bytes(&terminal)?,
        receipt_bytes: serialized_bytes(&receipt)?,
        terminal,
        receipt,
    })
}

pub(super) fn unchanged(
    fixture: &Fixture,
    prepared: &fixture::Prepared,
    ordinal: u8,
    original: &TerminalTruth,
) -> TestResult {
    let current = truth(fixture, prepared, ordinal)?;
    check!(eq; current.terminal, original.terminal);
    check!(eq; current.receipt, original.receipt);
    check!(eq; current.terminal_bytes, original.terminal_bytes);
    check!(eq; current.receipt_bytes, original.receipt_bytes);
    Ok(())
}

pub(super) fn no_successor(
    fixture: &Fixture,
    prepared: &fixture::Prepared,
    count: usize,
) -> TestResult {
    let prefix = format!("work/{}/attempt/", prepared.attempt.work_id());
    let next = format!(
        "{prefix}{}/",
        count.checked_add(1).ok_or("attempt accounting")?
    );
    let keys = fixture.store.journal_keys(PHASE)?;
    check!(eq; keys.iter().filter(|key| key.starts_with(&prefix) && key.ends_with("/reserved")).count(), count);
    check!(eq; keys.iter().filter(|key| key.starts_with(&prefix) && key.ends_with("/terminal")).count(), count);
    check!(!keys.iter().any(|key| key.starts_with(&next)));
    Ok(())
}

pub(super) fn note(outcome: &ResultsStageOutcome, category: &str, receipt: &Receipt) -> TestResult {
    check!(eq; outcome.per_source.len(), 1);
    let source = outcome.per_source.first().ok_or("source absent")?;
    check!(eq; source.slug, "milesplit");
    check!(eq; source.meets, 1);
    let prefix = format!("prepared recovery {category};");
    check!(source
        .notes
        .iter()
        .any(|note| note.contains(&prefix) && note.contains(&receipt.digest)));
    Ok(())
}

pub(super) fn retained(truth: &TerminalTruth, bytes: &[u8]) -> TestResult {
    let pin = read_pin(bytes)?;
    check!(eq; truth.terminal.attempt(), pin.attempt());
    check!(eq; truth.terminal.terminal(), &AcquisitionTerminal::ResponseRetained {
        prepared_digest: pin.prepared_digest().clone(),
        disposition: census_domain::model::CaptureDispositionV1::ReusableResponse,
    });
    Ok(())
}

pub(super) fn missing(truth: &TerminalTruth, bytes: &[u8]) -> TestResult {
    let pin = read_pin(bytes)?;
    let AcquisitionTerminal::PreparedRetentionFailed {
        prepared_digest,
        decision,
    } = truth.terminal.terminal()
    else {
        return Err("fixed loss terminal absent".into());
    };
    check!(eq; prepared_digest, pin.prepared_digest());
    check!(eq; decision.store_identity(), pin.store_identity());
    check!(eq; decision.slot_id(), pin.slot_id());
    check!(eq; decision.attempt(), pin.attempt());
    check!(eq; decision.prepared_digest(), pin.prepared_digest());
    check!(eq; decision.assessment().body(), census_domain::model::MissingLocation::BodyLeaf);
    check!(eq; decision.assessment().metadata(), census_domain::model::MissingMetadataV1::Missing {
        location: census_domain::model::MissingLocation::MetadataLeaf,
    });
    Ok(())
}

fn read_pin(bytes: &[u8]) -> TestResult<census_domain::model::PreparedRecoveryTarget> {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    let pins = value
        .get("pins")
        .and_then(serde_json::Value::as_array)
        .ok_or("pins absent")?;
    check!(eq; pins.len(), 1);
    Ok(serde_json::from_value(
        pins.first().ok_or("pin absent")?.clone(),
    )?)
}

pub(super) fn unavailable(outcome: &ResultsStageOutcome, receipt: &Receipt) -> TestResult {
    note(outcome, "retained_unavailable", receipt)?;
    let source = outcome.per_source.first().ok_or("source absent")?;
    check!(eq; source.rows, None);
    check!(eq; source.disposition, census_crawl::CollectionDisposition::Failed);
    check!(eq; source.errors, 1);
    check!(eq; source.unfinished, vec![super::super::fixture::RESULTS_URL.to_owned()]);
    check!(!outcome.is_terminal());
    Ok(())
}

pub(super) fn plan(bytes: &[u8], prepared: &fixture::Prepared, pins: usize) -> TestResult {
    let value: serde_json::Value = serde_json::from_slice(bytes)?;
    let actual_pins = value
        .get("pins")
        .and_then(serde_json::Value::as_array)
        .ok_or("pins absent")?;
    let targets = value
        .get("targets")
        .and_then(serde_json::Value::as_array)
        .ok_or("targets absent")?;
    check!(eq; actual_pins.len(), pins);
    check!(eq; targets.len(), 1);
    let target = targets.first().ok_or("target absent")?;
    check!(eq; target.get("work_id").and_then(serde_json::Value::as_str), Some(prepared.attempt.work_id().as_str()));
    if pins == 1 {
        let pin = read_pin(bytes)?;
        check!(eq; pin.attempt(), &prepared.attempt);
    } else {
        check!(eq; pins, 0);
        check!(eq; target.get("selection").and_then(serde_json::Value::as_str), Some("existing_no_prepared"));
    }
    Ok(())
}

pub(super) fn create_missing_archive_parents(
    fixture: &Fixture,
    prepared: &fixture::Prepared,
) -> TestResult {
    let root = fixture.store.http_cache_dir();
    let bodies = root.join("archive/bodies");
    let metadata = root
        .join("archive/captures")
        .join(prepared.input.content_digest().as_str());
    std::fs::create_dir_all(&bodies)?;
    std::fs::create_dir_all(&metadata)?;
    check!(!bodies
        .join(format!("{}.body", prepared.input.content_digest()))
        .exists());
    check!(!metadata
        .join(format!("{}.meta.json", prepared.input.metadata_digest()))
        .exists());
    Ok(())
}

pub(super) fn publish_exact(
    fixture: &Fixture,
    prepared: &fixture::Prepared,
) -> TestResult<PathBuf> {
    let root = fixture.store.http_cache_dir();
    let body = root
        .join("archive/bodies")
        .join(format!("{}.body", prepared.input.content_digest()));
    let metadata = root
        .join("archive/captures")
        .join(prepared.input.content_digest().as_str())
        .join(format!("{}.meta.json", prepared.input.metadata_digest()));
    std::fs::create_dir_all(body.parent().ok_or("body parent")?)?;
    std::fs::create_dir_all(metadata.parent().ok_or("metadata parent")?)?;
    std::fs::write(&body, &prepared.body)?;
    std::fs::write(metadata, prepared.input.metadata_bytes())?;
    Ok(body)
}

pub(super) fn saved_state(frames: &[Frame]) -> TestResult<JurisdictionState> {
    check!(eq; frames.iter().filter(|frame| frame.kind == 0x0403).count(), 1);
    let saved = frames
        .iter()
        .find(|frame| frame.kind == 0x0403)
        .ok_or("state save absent")?;
    check!(eq; saved.text(1)?, crate::restate_services::KEY_STATE);
    let bytes = codec::field(saved.bytes(3)?, 1)?.ok_or("state value absent")?;
    Ok(serde_json::from_slice(bytes)?)
}

pub(super) async fn invoke_later(
    client: &reqwest::Client,
    url: &str,
    fixture: &Fixture,
    journal: &[Frame],
) -> TestResult<Vec<Frame>> {
    let identity = b"results-v2-genuine-later-invocation";
    let mut start = codec::blob(1, identity)?;
    start.extend(codec::blob(2, identity)?);
    start.extend(codec::integer(3, u64::try_from(journal.len())?));
    let mut state = codec::blob(1, crate::restate_services::KEY_STATE.as_bytes())?;
    state.extend(codec::blob(2, &serde_json::to_vec(&fixture.state)?)?);
    start.extend(codec::blob(4, &state)?);
    start.extend(codec::blob(6, fixture.key().as_bytes())?);
    let mut body = Frame::new(0, start).encode()?;
    for frame in journal {
        body.extend(frame.encode()?);
    }
    let response = client
        .post(url)
        .header("content-type", "application/vnd.restate.invocation.v6")
        .body(body)
        .send()
        .await?;
    check!(eq; response.status(), reqwest::StatusCode::OK);
    let frames = codec::frames(&response.bytes().await?)?;
    if let Some(error) = frames.iter().find(|frame| frame.kind == 0x0002) {
        return Err(format!("later SDK invocation failed: {}", error.text(2)?).into());
    }
    Ok(frames)
}
