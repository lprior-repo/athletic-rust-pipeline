use crate::milesplit::SourceRowLocator;
use crate::participation_evidence::provider_target::{
    CompetitionCategoryEvidenceV1, SourceTargetPacketRefV1, SourceTargetPacketV1,
};
use crate::participation_evidence::recorded::*;
use crate::participation_evidence::result_ownership::read_retained_html_result_rows;
use crate::participation_evidence::{
    IndependentParticipationPacketV1, ParticipationEvidenceRefV1, RetainedBodyEncodingV1,
    RetainedHtmlCaptureV1, RetainedHttpProvenanceV1, SubjectOwnershipV1,
    PARTICIPATION_EVIDENCE_PHASE,
};
use crate::recording::{Recorded, RecordedJournal};
use crate::{CrawlError, Recording};
use census_domain::evidence_subject_attachment::{
    AttachmentReplayGrantV1, EvidenceSubjectAttachmentError,
};
use census_domain::model::Gender;
use census_store::{
    JournalReadBudget, ReadBudget, RetainedCaptureRootSchemaV1, RetainedCaptureRootV1, Store,
    StoreError, StoreSnapshot, Table,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod accepted_provider;
mod fixtures;
mod genuine_attachment;
mod html;
mod participation;
mod provider;
mod refusals;
mod roots;

use fixtures::*;

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn school_budget() -> ReadBudget {
    ReadBudget {
        rows: 65_536,
        bytes: 32 * 1024 * 1024,
        record_bytes: 2 * 1024 * 1024,
    }
}

fn expected_journal(phase: &str, key: &str, payload: Value) -> Recorded {
    Recorded {
        rows: Vec::new(),
        journal: vec![RecordedJournal {
            phase: phase.into(),
            key: key.into(),
            payload,
        }],
        effects: Vec::new(),
    }
}

fn retained_only_tables(store: &Store, expected: &str) -> TestResult {
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, expected);
    for table in [
        Table::Athletes,
        Table::Performances,
        Table::SourceObservations,
        Table::SourceIdentities,
        Table::AthleteIdentityDecisions,
        Table::RelayResults,
    ] {
        check!(eq; store.walk_table(table)?.rows, 0);
    }
    Ok(())
}

fn exact_root(
    snapshot: &StoreSnapshot<'_>,
    root: &RetainedCaptureRootV1,
    schema: RetainedCaptureRootSchemaV1,
    phase: &str,
    key: &str,
    payload: &Value,
) -> TestResult {
    check!(eq; root.schema, schema);
    check!(eq; root.schema.phase(), phase);
    check!(eq; root.key, key);
    let raw = snapshot
        .journal_payload_raw_bounded(phase, key, JournalReadBudget::new(2 * 1024 * 1024)?)?
        .ok_or("retained full payload")?;
    check!(eq; serde_json::from_str::<Value>(raw.get())?, payload.clone());
    check!(eq; root.payload.digest.as_str(), hash(raw.get().as_bytes()));
    check!(eq; root.payload.bytes, u64::try_from(raw.get().len())?);
    Ok(())
}

fn exact_capture(payload: &Value, field: Option<&str>, original: &[u8]) -> TestResult {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let capture = match field {
        Some(field) => payload.get(field).ok_or("retained capture field")?,
        None => payload,
    };
    let body = capture
        .get("body_base64")
        .and_then(Value::as_str)
        .ok_or("retained original body")?;
    check!(eq; STANDARD.decode(body)?, original);
    check!(eq; capture.get("body_bytes").and_then(Value::as_u64),
        Some(u64::try_from(original.len())?));
    check!(eq; capture.get("body_sha256").and_then(Value::as_str), Some(hash(original).as_str()));
    Ok(())
}

type ProtectedState = (
    u64,
    u64,
    u64,
    String,
    census_store::PublicIdentityInputRevisionsV2,
    census_domain::public_identity_history::RegistryHead,
);

fn protected_state(store: &Store) -> TestResult<ProtectedState> {
    let snapshot = store.snapshot();
    Ok((
        snapshot.evidence_generation(),
        snapshot.derived_generation(),
        store.receipt_count()?,
        snapshot.tables_digest(&Table::ALL)?,
        snapshot.public_identity_input_revisions_v2()?,
        snapshot
            .accepted_public_identity_snapshot()?
            .ok_or("current identity ledger")?
            .head()
            .clone(),
    ))
}

fn fresh_store() -> TestResult<(tempfile::TempDir, Store)> {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    drop(writer);
    no_public_birth(&store)?;
    Ok((directory, store))
}

fn reopen(directory: &tempfile::TempDir) -> TestResult<Store> {
    let (store, writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("store"))?.into_parts();
    drop(writer);
    Ok(store)
}

fn no_public_birth(store: &Store) -> TestResult {
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("current qualification ledger")?;
    check!(eq; accepted.head().sequence, 0);
    check!(eq; accepted.history_records().len(), 0);
    check!(eq; accepted.public_ids().count(), 0);
    Ok(())
}

fn non_observation_digest(store: &Store) -> TestResult<String> {
    let tables = Table::ALL
        .into_iter()
        .filter(|table| *table != Table::SourceObservations)
        .collect::<Vec<_>>();
    Ok(store.snapshot().tables_digest(&tables)?)
}

struct RetainedState {
    roots: Vec<(String, String, String)>,
    receipt: census_store::Receipt,
}

fn install_for_refusal(
    recorded: Recorded,
    store: &Store,
    prefix: &str,
) -> TestResult<RetainedState> {
    let journal = recorded.journal.clone();
    let applied = recorded.prepare(prefix)?.apply(store)?;
    let snapshot = store.snapshot();
    let roots = journal
        .iter()
        .map(|entry| -> TestResult<_> {
            let raw = snapshot
                .journal_payload_raw_bounded(
                    &entry.phase,
                    &entry.key,
                    JournalReadBudget::new(2 * 1024 * 1024)?,
                )?
                .ok_or("installed original root")?;
            check!(eq; serde_json::from_str::<Value>(raw.get())?, entry.payload);
            Ok((
                entry.phase.clone(),
                entry.key.clone(),
                raw.get().to_string(),
            ))
        })
        .collect::<TestResult<Vec<_>>>()?;
    let retained = RetainedState {
        roots,
        receipt: applied.receipt().clone(),
    };
    require_retained_state(store, &retained)?;
    Ok(retained)
}

fn require_retained_state(store: &Store, before: &RetainedState) -> TestResult {
    let snapshot = store.snapshot();
    for (phase, key, expected) in &before.roots {
        let raw = snapshot
            .journal_payload_raw_bounded(phase, key, JournalReadBudget::new(2 * 1024 * 1024)?)?
            .ok_or("retained original root")?;
        check!(eq; raw.get(), expected);
    }
    check!(eq; store.receipt(&before.receipt.operation)?, Some(before.receipt.clone()));
    Ok(())
}

mod provider_school;
