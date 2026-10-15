use std::io::{Read, Write};
use std::path::PathBuf;

use census_domain::model::{AthleteCandidateId, ReviewCase, ReviewVerdictRecord, StoreIdentity};
use census_domain::public_identity_history::local_adjudication::{
    ClaimTreatmentV1, LocalIdentitySplitV1, RetainedClaimDispositionV1,
};
use census_domain::public_identity_history::local_membership_withholding::LocalMembershipWithholdV1;
use census_domain::public_identity_history::{PublicAthleteId, RegistryHead};
use census_store::{PublicIdentityWriterV1, Store, Table};
use serde::{Deserialize, Serialize};

use super::{evidence, membership, physical, TestResult};

pub(super) const CHILD_NAME: &str = "manual_admission_graph_recovery_child";
pub(super) const CHILD_CASE: &str = "CENSUS_MANUAL_ADMISSION_CHILD_CASE";
pub(super) const CHILD_ROOT: &str = "CENSUS_MANUAL_ADMISSION_CHILD_ROOT";
const PLAN: &str = "manual-admission-recovery-plan.json";

pub(super) struct Plan {
    pub identity: StoreIdentity,
    pub snapshot_sequence: u64,
    pub head: RegistryHead,
    pub candidates: [AthleteCandidateId; 3],
    pub public_ids: [PublicAthleteId; 3],
    pub request: LocalIdentitySplitV1,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(super) enum Mode {
    Split,
    Withhold,
}

impl Mode {
    pub fn operation(self) -> &'static str {
        match self {
            Self::Split => "mixed-admission-partial-split",
            Self::Withhold => "mixed-admission-partial-withhold",
        }
    }

    pub fn other(self) -> Self {
        match self {
            Self::Split => Self::Withhold,
            Self::Withhold => Self::Split,
        }
    }

    pub fn kind(self) -> &'static str {
        match self {
            Self::Split => "manual",
            Self::Withhold => "withholding",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Requests {
    pub split: LocalIdentitySplitV1,
    pub withhold: LocalMembershipWithholdV1,
    mode: Mode,
}

impl Requests {
    pub fn prepare(
        &self,
        store: &Store,
        writer: &PublicIdentityWriterV1,
        mode: Mode,
        operation: &str,
    ) -> TestResult<Vec<u8>> {
        let head = writer.head(store)?;
        let expected = store.snapshot().revision();
        match mode {
            Mode::Split => Ok(writer
                .prepare_local_adjudication(store, operation, self.split.clone(), expected, &head)?
                .approval_bytes()?),
            Mode::Withhold => Ok(writer
                .prepare_membership_withholding(
                    store,
                    operation,
                    self.withhold.clone(),
                    expected,
                    &head,
                )?
                .approval_bytes()?),
        }
    }

    pub fn change_rationale(&mut self, mode: Mode) {
        match mode {
            Mode::Split => self.split.rationale.push_str(" changed"),
            Mode::Withhold => self.withhold.rationale.push_str(" changed"),
        }
    }
}

pub(super) struct Fixture {
    pub root: PathBuf,
    pub requests: Requests,
    pub mode: Mode,
    pub before: physical::State,
    pub identity: StoreIdentity,
    pub head: RegistryHead,
    pub evidence: u64,
    pub derived: u64,
}

impl Fixture {
    pub fn create(mode: Mode) -> TestResult<(tempfile::TempDir, Self)> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("mixed-admission");
        Store::create_public_identity_qualification_v3(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
        let requests = requests(&store, &writer, mode)?;
        requests.prepare(&store, &writer, mode.other(), mode.other().operation())?;
        write_plan(&root.join(PLAN), &requests)?;
        let identity = store.store_identity()?;
        let head = writer.head(&store)?;
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
                requests,
                mode,
                before,
                identity,
                head,
                evidence,
                derived,
            },
        ))
    }
}

fn requests(store: &Store, writer: &PublicIdentityWriterV1, mode: Mode) -> TestResult<Requests> {
    let mut split = evidence::install(store, writer)?;
    check!(eq; split.identity, store.store_identity()?);
    check!(eq; split.head, writer.head(store)?);
    check!(eq; split.snapshot_sequence, store.snapshot().sequence());
    check!(eq; split.candidates.len(), split.public_ids.len());
    let mut cases = store.snapshot().scan::<ReviewCase>(Table::ReviewCases)?;
    let mut verdicts = store
        .snapshot()
        .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
    let attachment = membership::install(store, writer)?;
    cases.extend(store.snapshot().scan::<ReviewCase>(Table::ReviewCases)?);
    verdicts.extend(
        store
            .snapshot()
            .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?,
    );
    store.replace_many(Table::ReviewCases, &cases)?;
    store.replace_many(Table::IdentityVerdicts, &verdicts)?;
    let withhold = membership::withholding(store, writer, &attachment)?;
    let inventory = writer.inspect_local_adjudication(
        store,
        "mixed-admission-split-inventory",
        &split.request.canonical,
    )?;
    split.request.affected_record_keys = inventory.affected_record_keys().to_vec();
    split.request.dispositions = inventory
        .claims()
        .iter()
        .map(|claim| RetainedClaimDispositionV1 {
            claim: claim.clone(),
            treatment: if claim.claim_path.starts_with("/source_links/")
                || claim.claim_path.starts_with("/identity_attestations/")
            {
                ClaimTreatmentV1::RejectIdentityCorroboration
            } else {
                ClaimTreatmentV1::RetainAsContext
            },
            rationale: "Synthetic operator separates previously merged retained identities".into(),
        })
        .collect();
    Ok(Requests {
        split: split.request,
        withhold,
        mode,
    })
}

fn write_plan(path: &std::path::Path, requests: &Requests) -> TestResult {
    let bytes = serde_json::to_vec(requests)?;
    check!(bytes.len() <= 1_048_576);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

pub(super) fn child() -> TestResult {
    check!(eq; std::env::var(CHILD_CASE)?.as_str(), CHILD_NAME);
    let root = PathBuf::from(std::env::var_os(CHILD_ROOT).ok_or("child root")?);
    check!(root.is_absolute());
    let path = root.join(PLAN);
    let metadata = std::fs::symlink_metadata(&path)?;
    check!(metadata.is_file() && metadata.len() > 0 && metadata.len() <= 1_048_576);
    let mut bytes = Vec::new();
    std::fs::File::open(path)?
        .take(1_048_577)
        .read_to_end(&mut bytes)?;
    check!(bytes.len() <= 1_048_576);
    let requests: Requests = serde_json::from_slice(&bytes)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    requests.prepare(&store, &writer, requests.mode, requests.mode.operation())?;
    Err("manual admission child returned without graph recovery interruption".into())
}
