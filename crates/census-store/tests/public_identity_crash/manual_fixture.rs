use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use census_domain::model::{AthleteCandidateId, StoreIdentity};
use census_domain::public_identity_history::local_adjudication::{
    LocalIdentityAdjudicationRecordV2, LocalIdentitySplitV1,
};
use census_domain::public_identity_history::{PublicAthleteId, RegistryHead};
use census_store::{PreparedLocalAdjudicationV1, PublicIdentityWriterV1, Store, Table};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::{evidence, physical, TestResult};

pub(super) const OPERATION: &str = "isolated-approved-manual-split-crash-v2";
pub(super) const CHILD_NAME: &str = "manual::public_identity_manual_crash_child_fixture";
const PLAN_FILE: &str = "manual-crash-plan.json";
const APPROVAL_FILE: &str = "manual-crash-approved.json";
const APPROVAL_SCHEMA: &str = "local-identity-adjudication-approval/v1";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Plan {
    pub identity: StoreIdentity,
    #[serde(deserialize_with = "decode_head")]
    pub head: RegistryHead,
    pub candidates: [AthleteCandidateId; 3],
    #[serde(deserialize_with = "decode_public_ids")]
    pub public_ids: [PublicAthleteId; 3],
    pub request: LocalIdentitySplitV1,
}

pub(super) struct Fixture {
    pub root: PathBuf,
    pub plan: Plan,
    pub before: physical::State,
    pub evidence: u64,
    pub derived: u64,
}

impl Fixture {
    pub fn create() -> TestResult<(tempfile::TempDir, Self)> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("identity");
        check!(root.is_absolute());
        Store::create_public_identity_qualification_v2(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v2(&root)?.into_parts();
        let plan = evidence::install(&store, &writer)?;
        check!(eq; plan.head.sequence, 4);
        check!(eq; store.snapshot().scan::<census_domain::model::AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?.len(), 4);
        let prepared = prepare(&store, &writer, &plan)?;
        check!(eq; prepared.record().expected_head, plan.head);
        drop(prepared);
        write_new(&root.join(PLAN_FILE), &serde_json::to_vec(&plan)?)?;
        let evidence = store.evidence_generation();
        let derived = store.derived_generation();
        store.flush()?;
        drop(writer);
        drop(store);
        let before = physical::read(&root)?;
        Ok((
            directory,
            Self {
                root,
                plan,
                before,
                evidence,
                derived,
            },
        ))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Approval {
    schema: String,
    actor: String,
    pub prepared: LocalIdentityAdjudicationRecordV2,
}

impl Approval {
    pub fn new(record: &LocalIdentityAdjudicationRecordV2) -> Self {
        Self {
            schema: APPROVAL_SCHEMA.into(),
            actor: "local_operator".into(),
            prepared: record.clone(),
        }
    }

    pub fn encoded(&self) -> TestResult<(Vec<u8>, String)> {
        check!(eq; self.schema, APPROVAL_SCHEMA);
        check!(eq; self.actor, "local_operator");
        let bytes = serde_json::to_vec(self)?;
        let mut digest = Sha256::new();
        digest.update(APPROVAL_SCHEMA.as_bytes());
        digest.update([0]);
        digest.update(&bytes);
        Ok((bytes, format!("{:x}", digest.finalize())))
    }
}

pub(super) fn prepare<'s>(
    store: &'s Store,
    writer: &PublicIdentityWriterV1,
    plan: &Plan,
) -> TestResult<PreparedLocalAdjudicationV1<'s>> {
    let original_sequence = store.snapshot().sequence();
    let prepared = writer.prepare_local_adjudication(
        store,
        OPERATION,
        plan.request.clone(),
        store.snapshot().revision(),
        &plan.head,
    )?;
    check!(eq; prepared.record().snapshot_sequence, original_sequence);
    Ok(prepared)
}

pub(super) fn child() -> TestResult {
    check!(eq; std::env::var(super::super::fixture::CHILD_CASE)?.as_str(), CHILD_NAME);
    let root = PathBuf::from(
        std::env::var_os(super::super::fixture::CHILD_ROOT).ok_or("manual child root")?,
    );
    check!(root.is_absolute());
    let plan: Plan = read_json(&root.join(PLAN_FILE))?;
    let (store, writer) = Store::open_public_identity_qualification_v2(&root)?.into_parts();
    check!(eq; writer.store_identity(), &plan.identity);
    check!(eq; writer.head(&store)?, plan.head);
    check!(eq; store.snapshot().scan::<census_domain::model::AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?.len(), 4);
    let prepared = prepare(&store, &writer, &plan)?;
    let (bytes, digest) = Approval::new(prepared.record()).encoded()?;
    write_new(&root.join(APPROVAL_FILE), &bytes)?;
    writer.apply_local_adjudication(&store, prepared, bytes, digest)?;
    Err("manual child returned without interruption at the durable boundary".into())
}

pub(super) fn read_approval(root: &Path) -> TestResult<Approval> {
    read_json(&root.join(APPROVAL_FILE))
}

fn write_new(path: &Path, bytes: &[u8]) -> TestResult {
    check!(bytes.len() <= 1_048_576);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    Ok(())
}

fn read_json<T: DeserializeOwned>(path: &Path) -> TestResult<T> {
    let metadata = std::fs::symlink_metadata(path)?;
    check!(metadata.is_file() && metadata.len() > 0 && metadata.len() <= 1_048_576);
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(1_048_577)
        .read_to_end(&mut bytes)?;
    check!(bytes.len() <= 1_048_576);
    Ok(serde_json::from_slice(&bytes)?)
}

fn decode_head<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<RegistryHead, D::Error> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct HeadInput {
        sequence: u64,
        digest: String,
    }
    let input = HeadInput::deserialize(deserializer)?;
    Ok(RegistryHead {
        sequence: input.sequence,
        digest: input.digest,
    })
}

fn decode_public_ids<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<[PublicAthleteId; 3], D::Error> {
    let [first, second, third] = <[String; 3]>::deserialize(deserializer)?;
    Ok([
        PublicAthleteId::parse(&first).map_err(serde::de::Error::custom)?,
        PublicAthleteId::parse(&second).map_err(serde::de::Error::custom)?,
        PublicAthleteId::parse(&third).map_err(serde::de::Error::custom)?,
    ])
}
