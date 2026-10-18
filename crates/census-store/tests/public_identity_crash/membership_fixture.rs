use super::{evidence, physical, TestResult};
use crate::fixture_files::{read_json, write_new};
use census_domain::public_identity_history::local_membership_withholding::{
    LocalMembershipWithholdV1, LocalMembershipWithholdingRecordV2, MembershipAttachmentV1,
};
use census_domain::public_identity_history::{HistoryRequest, PublicResolution, RegistryHead};
use census_store::{PublicIdentityWriterV1, SourceCandidateMembershipRequestV1, Store};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub(super) const CHILD_NAME: &str = "membership::membership_crash_child_fixture";
const PLAN: &str = "membership-crash-plan.json";
const ATTACH: &str = "native-membership-attachment";
const WITHHOLD: &str = "native-membership-withholding";
const APPROVAL_SCHEMA: &str = "local-source-membership-withholding-approval/v2";
#[derive(Clone, Copy)]
pub(super) enum Mode {
    Attach,
    Withhold,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Plan {
    pub attachment: SourceCandidateMembershipRequestV1,
    action: Action,
}
#[derive(Serialize, Deserialize)]
enum Action {
    Attach {
        sequence: u64,
        digest: String,
    },
    Withhold {
        record: Box<LocalMembershipWithholdingRecordV2>,
        graph: census_store::GraphAuthorityReferenceV1,
    },
}
pub(super) struct Fixture {
    pub root: PathBuf,
    pub plan: Plan,
    pub before: physical::State,
    pub owner: PublicResolution,
    pub source_record: Vec<u8>,
    pub expected: super::oracle::Expected,
}
impl Fixture {
    pub fn create(mode: Mode) -> TestResult<(tempfile::TempDir, Self)> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("membership");
        Store::create_public_identity_qualification_v3(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        writer.enroll_graph_admission_v1(&store, store.snapshot().revision())?;
        let attachment = evidence::install(&store, &writer)?;
        let head = writer.head(&store)?;
        let action = match mode {
            Mode::Attach => Action::Attach {
                sequence: head.sequence,
                digest: head.digest,
            },
            Mode::Withhold => prepare_withholding(&store, &writer, &attachment)?,
        };
        let plan = Plan { attachment, action };
        let expected = super::oracle::Expected::capture(&store, &writer)?;
        let snapshot = store.snapshot();
        let accepted = snapshot
            .accepted_public_identity_snapshot()?
            .ok_or("accepted fixture")?;
        let owner = accepted.resolve_public(&plan.attachment.owner)?;
        let source_record = serde_json::to_vec(
            accepted
                .history_records()
                .iter()
                .find(|record| {
                    matches!(&record.request, HistoryRequest::Establish { binding, .. }
                    if binding.id == plan.attachment.anchor_binding_id)
                })
                .ok_or("source birth anchor")?,
        )?;
        write_new(&root.join(PLAN), &serde_json::to_vec(&plan)?)?;
        drop(accepted);
        drop(snapshot);
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
                owner,
                source_record,
                expected,
            },
        ))
    }
}

fn prepare_withholding(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    attachment: &SourceCandidateMembershipRequestV1,
) -> TestResult<Action> {
    writer.extend_source_candidate(
        store,
        ATTACH,
        attachment,
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("attached snapshot")?;
    let record = accepted
        .history_records()
        .last()
        .ok_or("attachment record")?;
    let HistoryRequest::BindCandidateMembership { membership, .. } = &record.request else {
        return Err("attachment kind".into());
    };
    let target = MembershipAttachmentV1 {
        sequence: record.sequence,
        digest: record.digest.clone(),
        membership_id: membership.id.clone(),
    };
    let inventory = writer.inspect_membership_withholding(
        store,
        "membership-crash-withhold-inventory",
        &attachment.owner,
    )?;
    let request = LocalMembershipWithholdV1 {
        case_id: attachment.case_id.clone(),
        canonical: attachment.owner.clone(),
        expected_withheld_memberships: writer.preview_membership_withholding(
            store,
            &attachment.owner,
            &target,
        )?,
        attachment: target,
        affected_record_keys: inventory.affected_record_keys().to_vec(),
        rationale: "Synthetic operator withholds exact attachment".into(),
    };
    let prepared = writer.prepare_membership_withholding(
        store,
        WITHHOLD,
        request,
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    Ok(Action::Withhold {
        record: Box::new(prepared.record().clone()),
        graph: prepared.graph().clone(),
    })
}

impl Plan {
    pub fn withholding(&self) -> Option<&LocalMembershipWithholdingRecordV2> {
        match &self.action {
            Action::Attach { .. } => None,
            Action::Withhold { record, .. } => Some(record),
        }
    }
    pub fn withholding_graph(&self) -> TestResult<&census_store::GraphAuthorityReferenceV1> {
        match &self.action {
            Action::Withhold { graph, .. } => Ok(graph),
            Action::Attach { .. } => Err("attachment has no manual approval".into()),
        }
    }
    pub fn operation(&self) -> &'static str {
        match &self.action {
            Action::Attach { .. } => ATTACH,
            Action::Withhold { .. } => WITHHOLD,
        }
    }
    pub fn prior_head(&self) -> RegistryHead {
        match &self.action {
            Action::Attach { sequence, digest } => RegistryHead {
                sequence: *sequence,
                digest: digest.clone(),
            },
            Action::Withhold { record, .. } => record.expected_head.clone(),
        }
    }
    pub fn apply(
        &self,
        store: &Store,
        writer: &PublicIdentityWriterV1,
    ) -> TestResult<(RegistryHead, bool)> {
        match &self.action {
            Action::Attach { .. } => {
                let outcome = writer.extend_source_candidate(
                    store,
                    ATTACH,
                    &self.attachment,
                    store.snapshot().revision(),
                    &self.prior_head(),
                )?;
                Ok((outcome.head().clone(), outcome.replayed()))
            }
            Action::Withhold { record, graph } => {
                let (bytes, digest) = approval(record, graph)?;
                if let Some(outcome) =
                    writer.replay_membership_withholding(store, WITHHOLD, &digest)?
                {
                    return Ok((outcome.head().clone(), outcome.replayed()));
                }
                let prepared = writer.prepare_membership_withholding(
                    store,
                    WITHHOLD,
                    record.request.clone(),
                    store.snapshot().revision(),
                    &record.expected_head,
                )?;
                check!(eq; prepared.record(), record.as_ref());
                check!(eq; prepared.graph(), graph);
                check!(eq; prepared.approval_bytes()?, bytes);
                check!(eq; prepared.approval_digest()?, digest);
                let outcome =
                    writer.apply_membership_withholding(store, prepared, bytes, digest)?;
                Ok((outcome.head().clone(), outcome.replayed()))
            }
        }
    }
}

pub(super) fn child() -> TestResult {
    check!(eq; std::env::var(super::super::fixture::CHILD_CASE)?.as_str(), CHILD_NAME);
    let root =
        PathBuf::from(std::env::var_os(super::super::fixture::CHILD_ROOT).ok_or("child root")?);
    let plan: Plan = read_json(&root.join(PLAN))?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    write_new(
        &root.join("child-precommit-sequence.bin"),
        &store.snapshot().sequence().to_be_bytes(),
    )?;
    plan.apply(&store, &writer)?;
    Err("membership child returned without reaching boundary".into())
}

pub(super) fn approval(
    record: &LocalMembershipWithholdingRecordV2,
    graph: &census_store::GraphAuthorityReferenceV1,
) -> TestResult<(Vec<u8>, String)> {
    #[derive(Serialize)]
    struct Approval<'a> {
        schema: &'static str,
        actor: &'static str,
        prepared: &'a LocalMembershipWithholdingRecordV2,
        graph: &'a census_store::GraphAuthorityReferenceV1,
    }
    check!(eq; graph.domain_reference()?, record.graph);
    let bytes = serde_json::to_vec(&Approval {
        schema: APPROVAL_SCHEMA,
        actor: "local_operator",
        prepared: record,
        graph,
    })?;
    check!(bytes.len() <= 1_048_576);
    let mut digest = Sha256::new();
    digest.update(APPROVAL_SCHEMA.as_bytes());
    digest.update([0]);
    digest.update(&bytes);
    Ok((bytes, format!("{:x}", digest.finalize())))
}
