use super::context;
use census_crawl::participation_evidence::{
    RetainedBodyEncodingV1, RetainedHtmlCaptureV1, RetainedHttpProvenanceV1,
};
use census_domain::evidence_subject_attachment::*;
use census_domain::model::{
    CanonicalAthlete, Evidence, Gender, GradYear, SchoolId, SourceIdentity, SourceNamespace,
    SourceRef,
};
use census_domain::public_identity_history::RegistryHead;
use census_store::{
    AttachmentInspectionRequestV1, Receipt, RetainedCaptureLeafKindV1, RetainedCaptureLeafV1,
    RetainedCaptureRootSchemaV1, RetainedCaptureSlotV1, Store, Table,
};
use clap::Parser;
use sha2::{Digest, Sha256};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

pub(in crate::cli::capture_protocol::attachment_review) type TestResult<T = ()> =
    Result<T, Box<dyn std::error::Error + Send + Sync>>;
const BODY: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../census-crawl/src/participation_evidence/result_ownership/tests/fixtures/milesplit-result.html"));
const HEADERS: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../census-crawl/src/participation_evidence/result_ownership/tests/fixtures/milesplit-result.headers"));
const URL: &str =
    "https://mi.milesplit.com/meets/763888-mhsaa-lp-region-09-1-milford-2026/results/1304896/raw";

pub(in crate::cli::capture_protocol::attachment_review) fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(in crate::cli::capture_protocol::attachment_review) struct Case {
    pub directory: tempfile::TempDir,
    pub root: PathBuf,
    pub command_path: PathBuf,
    pub command: RetainedEvidenceSubjectAttachmentCommandV1,
    pub inline: Vec<PathBuf>,
    pub retained: Vec<PathBuf>,
}

impl Case {
    pub fn new(retained_body: bool) -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("store");
        Store::create_current_qualification_v3(&root)?;
        let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
        let revision = store.snapshot().revision();
        writer.enroll_graph_admission_v1(&store, revision)?;
        let school = SchoolId::mint("sch", &["controlled-cli-attachment"]);
        let mut athlete = CanonicalAthlete::new(
            &school,
            "Synthetic CLI attachment runner",
            GradYear::CO2027,
            Gender::Girls,
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, "93101"),
        )?;
        athlete.evidence.push(Evidence::parsed(
            SourceRef::new(
                "controlled-cli-attachment",
                Some("https://mi.milesplit.com/athletes/93101".into()),
            ),
            "2026-10-14",
        ));
        store.append_many(Table::Athletes, std::slice::from_ref(&athlete))?;
        let revision = store.snapshot().revision();
        let head = writer.head(&store)?;
        let accepted = writer.establish_source_bound(
            &store,
            "controlled-cli-target-birth",
            &athlete.id.cast(),
            revision,
            &head,
        )?;
        let objects = context::objects(BODY)?;
        let leaf = if retained_body {
            Some(retain_body(&store)?)
        } else {
            None
        };
        let inspection = store.inspect_evidence_subject_attachment_v1(
            AttachmentInspectionRequestV1::Attach {
                target: accepted.public_id().clone(),
                prior: AttachmentPredecessorV1::First,
            },
        )?;
        let command = context::command(&objects, &inspection)?;
        let command_path = directory.path().join("command.json");
        std::fs::write(&command_path, serde_json::to_vec(&command)?)?;
        let mut inline = Vec::new();
        for (index, object) in objects.iter().enumerate() {
            if leaf
                .as_ref()
                .is_some_and(|leaf| leaf.object == object.reference)
            {
                continue;
            }
            let path = directory.path().join(format!("inline-{index}.bin"));
            std::fs::write(&path, &object.bytes)?;
            inline.push(path);
        }
        let mut retained = Vec::new();
        if let Some(leaf) = leaf {
            let path = directory.path().join("original-body-leaf.json");
            std::fs::write(&path, serde_json::to_vec(&leaf)?)?;
            retained.push(path);
        }
        drop(inspection);
        drop(writer);
        drop(store);
        Ok(Self {
            directory,
            root,
            command_path,
            command,
            inline,
            retained,
        })
    }

    pub fn write_command(
        &self,
        command: &RetainedEvidenceSubjectAttachmentCommandV1,
    ) -> TestResult {
        Ok(std::fs::write(
            &self.command_path,
            serde_json::to_vec(command)?,
        )?)
    }
}

fn retain_body(store: &Store) -> TestResult<RetainedCaptureLeafV1> {
    check!(!BODY.is_empty());
    let acquired = chrono::DateTime::parse_from_rfc3339("2026-10-16T00:06:57Z")?.to_utc();
    let capture = RetainedHtmlCaptureV1::new(
        URL,
        URL,
        200,
        acquired,
        BODY,
        RetainedHttpProvenanceV1 {
            request_method: "GET".into(),
            request_headers: vec![
                ("User-Agent".into(), "OpenAI-research/0.1".into()),
                ("Accept".into(), "*/*".into()),
            ],
            response_header_bytes: HEADERS.to_vec(),
            content_type: "text/html; charset=UTF-8".into(),
            body_encoding: RetainedBodyEncodingV1::Identity,
            started_at: chrono::DateTime::parse_from_rfc3339("2026-10-16T00:06:38Z")?.to_utc(),
        },
    )?;
    let schema = RetainedCaptureRootSchemaV1::HtmlResultCaptureV1;
    store.journal_done(schema.phase(), capture.body_sha256(), &capture)?;
    let view = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let held = view
        .read_retained_capture_root_v1(schema, capture.body_sha256(), &mut grant)?
        .ok_or("retained original HTML capture")?;
    let leaf = RetainedCaptureLeafV1 {
        version: 1,
        root: held.root().clone(),
        slot: RetainedCaptureSlotV1::SelfCapture,
        kind: RetainedCaptureLeafKindV1::OriginalBody,
        object: capture.original_body_reference()?,
    };
    check!(eq; leaf.object.bytes, u64::try_from(BODY.len())?);
    check!(eq; leaf.object.digest.as_str(), sha256(BODY));
    view.verify_retained_capture_leaf_v1(&leaf, &mut grant)?;
    Ok(leaf)
}

#[derive(Debug, PartialEq, Eq)]
pub(in crate::cli::capture_protocol::attachment_review) struct State {
    pub sequence: u64,
    pub receipt_count: u64,
    pub tables: String,
    pub counts: Vec<(String, u64)>,
    pub head: RegistryHead,
    pub inventory: AttachmentInventoryV1,
    pub records: Vec<ReviewedReferenceRecordV1>,
    pub receipts: Vec<Receipt>,
}

pub(in crate::cli::capture_protocol::attachment_review) fn state(root: &Path) -> TestResult<State> {
    let (store, _writer) = Store::open_current_qualification_v3(root)?.into_parts();
    let view = store.snapshot();
    let accepted = view
        .accepted_public_identity_snapshot()?
        .ok_or("accepted target ledger")?;
    let mut grant = AttachmentReplayGrantV1::new();
    let references = view.read_evidence_subject_attachment_references_v1(&mut grant)?;
    Ok(State {
        sequence: view.sequence(),
        receipt_count: store.receipt_count()?,
        tables: view.tables_digest(&Table::ALL)?,
        counts: view.appended_counts()?,
        head: accepted.head().clone(),
        inventory: references.inventory().clone(),
        records: references.records().to_vec(),
        receipts: references.receipts().to_vec(),
    })
}

pub(in crate::cli::capture_protocol::attachment_review) fn invoke(
    case: &Case,
    verb: &str,
    approval: Option<&str>,
    with_materials: bool,
) -> anyhow::Result<Vec<u8>> {
    let mut output = Vec::new();
    invoke_to(case, verb, approval, with_materials, &mut output)?;
    Ok(output)
}

pub(in crate::cli::capture_protocol::attachment_review) fn invoke_to(
    case: &Case,
    verb: &str,
    approval: Option<&str>,
    with_materials: bool,
    output: &mut Vec<u8>,
) -> anyhow::Result<()> {
    let mut args = vec![
        OsString::from("census-service"),
        OsString::from("--store"),
        case.root.as_os_str().to_owned(),
        OsString::from("capture-protocol"),
        OsString::from(verb),
        OsString::from("--command"),
        case.command_path.as_os_str().to_owned(),
    ];
    if with_materials {
        for (flag, paths) in [
            ("--inline", &case.inline),
            ("--retained-leaf", &case.retained),
        ] {
            for path in paths {
                args.extend([OsString::from(flag), path.as_os_str().to_owned()]);
            }
        }
    }
    if let Some(approval) = approval {
        args.extend([OsString::from("--approve-sha256"), OsString::from(approval)]);
    }
    let cli = crate::cli::Cli::try_parse_from(args)?;
    let crate::cli::command::Command::CaptureProtocol(args) = &cli.command else {
        anyhow::bail!("attachment command did not parse as capture-protocol");
    };
    crate::cli::capture_protocol::run_to(cli.store.as_deref(), args, output)
}
