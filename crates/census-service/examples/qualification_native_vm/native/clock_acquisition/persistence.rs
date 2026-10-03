use super::{
    artifacts, checks, CapturePaths, FetchOutcome, FetchStats, Identity, JournalRef, Phase, Record,
    Session, Store, JOURNAL, URL,
};
use anyhow::{ensure, Context, Result};
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn previous(
    store: &Store,
    root: &Path,
    identity: &Identity,
    phase: Phase,
) -> Result<Option<Record>> {
    ensure!(
        !store.journal_contains(JOURNAL, &journal_key(identity, phase))?
            && !root
                .join(format!("out/{}.json", phase.label()))
                .try_exists()?,
        "phase already acquired; preserve its original physical evidence"
    );
    if phase == Phase::Before {
        ensure!(
            !store.journal_contains(JOURNAL, &journal_key(identity, Phase::After))?,
            "after phase exists without before"
        );
        return Ok(None);
    }
    let value = store
        .journal_payload(JOURNAL, &journal_key(identity, Phase::Before))?
        .context("after requires the original committed before acquisition")?;
    let mut record: Record = serde_json::from_value(value.clone())?;
    ensure!(
        record.identity == *identity,
        "before acquisition belongs to another immutable manifest"
    );
    ensure!(
        record.journal.store_root == root
            && record.journal.phase == JOURNAL
            && record.journal.key == journal_key(identity, Phase::Before)
            && record.journal.readback == root.join("out/before.json"),
        "before journal reference mismatch"
    );
    ensure!(
        artifacts::json(&record.journal.readback)? == value,
        "before journal artifact changed"
    );
    validate_paths(root, &record)?;
    record.capture.body = artifacts::read(&record.paths.body)?;
    verify_preserved(&record)?;
    let counts = checks::capture(&record.capture, &record.stats)?;
    ensure!(
        counts == (record.schools, record.xc_tf_appointments),
        "before directory readback changed"
    );
    checks::acquisition_time(&record.capture, &record.clock_before, &record.clock)?;
    Ok(Some(record))
}

pub(super) fn finish(
    session: &Session,
    phase: Phase,
    clock_before: Value,
    acquired: Result<(FetchOutcome, FetchStats)>,
) -> Result<Value> {
    let clock = super::super::clock::clock()?;
    let result = save(session, phase, &clock_before, &clock, acquired);
    if let Err(error) = &result {
        artifacts::append(
            &session.root.join("out/failures.jsonl"),
            &json!({"phase":phase,"identity":session.identity,"clock_before":clock_before,
                "clock":clock,"error":format!("{error:#}")}),
        )?;
    }
    result
}

fn save(
    session: &Session,
    phase: Phase,
    clock_before: &Value,
    clock: &Value,
    acquired: Result<(FetchOutcome, FetchStats)>,
) -> Result<Value> {
    let (capture, stats) = acquired?;
    let paths = preserve(&session.root, phase, &capture)?;
    let (schools, xc_tf_appointments) = checks::capture(&capture, &stats)?;
    checks::acquisition_time(&capture, clock_before, clock)?;
    let record = Record {
        phase,
        identity: session.identity.clone(),
        capture,
        paths,
        journal: JournalRef {
            store_root: session.root.clone(),
            phase: JOURNAL.to_owned(),
            key: journal_key(&session.identity, phase),
            readback: session.root.join(format!("out/{}.json", phase.label())),
        },
        clock_before: clock_before.clone(),
        clock: clock.clone(),
        stats,
        schools,
        xc_tf_appointments,
    };
    verify_preserved(&record)?;
    if let Some(before) = &session.previous {
        checks::crossing(before, &record)?;
    }
    session
        .store
        .journal_done(JOURNAL, &record.journal.key, &record)?;
    let readback = session
        .store
        .journal_payload(JOURNAL, &record.journal.key)?
        .context("committed acquisition journal disappeared")?;
    ensure!(
        readback == serde_json::to_value(&record)?,
        "acquisition journal readback mismatch"
    );
    artifacts::write(
        &record.journal.readback,
        &serde_json::to_vec_pretty(&readback)?,
    )?;
    Ok(readback)
}

fn preserve(root: &Path, phase: Phase, capture: &FetchOutcome) -> Result<CapturePaths> {
    let request_digest = artifacts::sha(format!("GET\u{1f}{URL}\u{1f}").as_bytes());
    let key = request_digest
        .get(..32)
        .context("production cache key too short")?;
    let metadata = artifacts::read(&root.join(format!("http/{key}.meta.json")))?;
    validate_metadata(&serde_json::from_slice(&metadata)?, capture)?;
    let metadata_sha256 = artifacts::sha(&metadata);
    let paths = CapturePaths {
        body: root.join(format!("out/{}.body", phase.label())),
        metadata: root.join(format!("out/{}.capture.json", phase.label())),
        archive_body: root.join(format!(
            "http/archive/bodies/{}.body",
            capture.content_digest
        )),
        archive_metadata: root.join(format!(
            "http/archive/captures/{}/{}.meta.json",
            capture.content_digest, metadata_sha256
        )),
        metadata_sha256,
    };
    ensure!(
        artifacts::read(&paths.archive_body)? == capture.body,
        "production archived body differs from physical response"
    );
    ensure!(
        artifacts::read(&paths.archive_metadata)? == metadata,
        "production archived metadata differs from original capture"
    );
    artifacts::write(&paths.body, &capture.body)?;
    artifacts::write(&paths.metadata, &metadata)?;
    Ok(paths)
}

fn verify_preserved(record: &Record) -> Result<()> {
    ensure!(
        artifacts::read(&record.paths.body)? == record.capture.body
            && artifacts::read(&record.paths.archive_body)? == record.capture.body,
        "retained raw physical body mismatch"
    );
    let metadata = artifacts::read(&record.paths.metadata)?;
    ensure!(
        artifacts::sha(&metadata) == record.paths.metadata_sha256
            && artifacts::read(&record.paths.archive_metadata)? == metadata,
        "retained original capture metadata mismatch"
    );
    validate_metadata(&serde_json::from_slice(&metadata)?, &record.capture)
}

fn validate_metadata(metadata: &Value, capture: &FetchOutcome) -> Result<()> {
    let outcome = serde_json::to_value(capture)?;
    [
        "url",
        "method",
        "status",
        "content_digest",
        "bytes",
        "fetched_at",
    ]
    .into_iter()
    .try_for_each(|name| -> Result<()> {
        ensure!(
            metadata.get(name) == outcome.get(name),
            "original capture metadata {name} mismatch"
        );
        Ok(())
    })?;
    ensure!(
        metadata.get("content_type").and_then(Value::as_str) == capture.content_type.as_deref(),
        "capture content type mismatch"
    );
    Ok(())
}

fn validate_paths(root: &Path, record: &Record) -> Result<()> {
    let paths = &record.paths;
    ensure!(
        paths.body == root.join("out/before.body")
            && paths.metadata == root.join("out/before.capture.json")
            && paths.archive_body
                == root.join(format!(
                    "http/archive/bodies/{}.body",
                    record.capture.content_digest
                ))
            && paths.archive_metadata
                == root.join(format!(
                    "http/archive/captures/{}/{}.meta.json",
                    record.capture.content_digest, paths.metadata_sha256
                )),
        "before capture references leave the owning acquisition store"
    );
    Ok(())
}

fn journal_key(identity: &Identity, phase: Phase) -> String {
    format!("{}:{}", identity.source_unit, phase.label())
}

pub(super) fn ensure_owned_directories(root: &Path) -> Result<()> {
    ["fjall", "http", "out"].into_iter().try_for_each(|name| {
        let path = root.join(name);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => {
                ensure!(
                    metadata.is_dir() && !metadata.file_type().is_symlink(),
                    "acquisition directory must not alias another store: {}",
                    path.display()
                );
                ensure!(
                    std::fs::canonicalize(&path)? == path,
                    "acquisition directory resolves outside its owning root"
                );
                Ok(())
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    })
}
