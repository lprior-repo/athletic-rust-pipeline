use super::*;
use census_domain::evidence_subject_attachment::*;
use census_domain::model::{CanonicalAthlete, Evidence, Gender, GradYear, SchoolId, SourceIdentity, SourceNamespace, SourceRef};
use census_domain::public_identity_history::PublicAthleteId;
use census_store::{Store, Table, PublicIdentityWriterV1, EvidenceSubjectAttachmentInspectionV1,
    AttachmentInspectionRequestV1, EvidenceSubjectAttachmentPreparationV1, Application};
use std::path::{Path, PathBuf};
use std::collections::BTreeMap;
use sha2::{Digest, Sha256};

mod context;
mod observe;
use context::Context;

#[derive(Clone, Copy)]
pub(super) enum TargetCase { Same, Other }

pub(in crate::participation_evidence::recorded) fn verify_targets(closed_root: &Path) -> TestResult {
    let original = files(closed_root)?;
    for case in [TargetCase::Same, TargetCase::Other] {
        let result = scenario(closed_root, case);
        check!(eq; files(closed_root)?, original);
        result?;
    }
    Ok(())
}

fn scenario(closed_root: &Path, case: TargetCase) -> TestResult {
    let directory = tempfile::tempdir()?;
    let backup = directory.path().join("backup");
    let root = directory.path().join("private-store");
    Store::backup(closed_root, &backup)?;
    Store::restore(&backup, &root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    let expected = observe::current(&store, TargetCase::Other)?;
    let original_target = target(&store)?;
    let selected = match case {
        TargetCase::Same => original_target,
        TargetCase::Other => second_target(&store, &writer)?,
    };
    append_unsupported(&store, &selected)?;
    let before = store.snapshot().tables_digest(&Table::ALL)?;
    let receipts = store.receipt_count()?;
    let found = observe::current(&store, case)?;
    let frozen = observe::frozen(&store, case)?;
    match case {
        TargetCase::Same => { check!(found.is_empty()); check!(frozen.is_empty()); }
        TargetCase::Other => { check!(eq; found, expected); check!(eq; frozen, expected); }
    }
    check!(eq; store.snapshot().tables_digest(&Table::ALL)?, before);
    check!(eq; store.receipt_count()?, receipts);
    drop(writer);
    drop(store);
    directory.close()?;
    Ok(())
}

fn target(store: &Store) -> TestResult<PublicAthleteId> {
    let snapshot = store.snapshot();
    let mut grant = Grant::new();
    let owner = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let metadata = owner.discover_active_subjects_v1(AttachmentSubjectRoleV1::AthleteStatus, &mut grant)?;
    let [item] = metadata.subjects() else { return Err("one genuine status subject".into()); };
    let bound = owner.bind_exact_subject(item.subject(), &mut grant)?.ok_or("active genuine status")?;
    bound.require_original_grant(&mut grant)?;
    Ok(bound.target().public_id.clone())
}

fn second_target(store: &Store, writer: &PublicIdentityWriterV1) -> TestResult<PublicAthleteId> {
    let school = SchoolId::mint("sch", &["synthetic-unsupported-status-isolation"]);
    let mut athlete = CanonicalAthlete::new(&school, "Synthetic Runner Unsupported Status",
        GradYear::CO2027, Gender::Girls, SourceIdentity::new(SourceNamespace::MilesplitAthlete, "9931002"))?;
    athlete.evidence.push(Evidence::parsed(SourceRef::new("synthetic-unsupported-status-test",
        Some("https://mi.milesplit.com/athletes/9931002".into())), "2026-10-14"));
    store.append(Table::Athletes, &athlete)?;
    let outcome = writer.establish_source_bound(store, "synthetic-unsupported-status-second-target",
        &athlete.id.cast(), store.snapshot().revision(), &writer.head(store)?)?;
    check!(!outcome.replayed());
    Ok(outcome.public_id().clone())
}

fn append_unsupported(store: &Store, target: &PublicAthleteId) -> TestResult {
    let inspection = store.inspect_evidence_subject_attachment_v1(AttachmentInspectionRequestV1::Attach {
        target: target.clone(), prior: AttachmentPredecessorV1::First,
    })?;
    let context = Context::synthetic("synthetic-unsupported-status-isolation")?;
    let command = context.command("synthetic-unsupported-status-append", &inspection)?;
    let objects = context.borrowed();
    let prepared = store.prepare_evidence_subject_attachment_v1(command,
        EvidenceSubjectAttachmentPreparationV1::New { inspection, context: &objects })?;
    check!(!prepared.is_replay());
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    check!(matches!(store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest)?, Application::Written(_)));
    Ok(())
}

fn text(value: &str) -> TestResult<AttachmentTextV1> { Ok(AttachmentTextV1::parse(value)?) }
fn list<T>(value: Vec<T>) -> TestResult<AttachmentListV1<T>> { Ok(AttachmentListV1::new(value)?) }
fn reference(value: &[u8]) -> TestResult<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 { digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(value)))?, bytes: u64::try_from(value.len())? })
}

const MAX_FILES: usize = 4096;
const MAX_BYTES: u64 = 512 * 1024 * 1024;
const CHUNK: usize = 64 * 1024;

fn files(root: &Path) -> TestResult<BTreeMap<PathBuf, (u64, String)>> {
    let mut pending = vec![root.to_path_buf()];
    let mut discovered = 1usize;
    let mut total = 0u64;
    let mut output = BTreeMap::new();
    for _ in 0..MAX_FILES {
        let Some(path) = pending.pop() else { return Ok(output); };
        let metadata = std::fs::symlink_metadata(&path)?;
        check!(!metadata.file_type().is_symlink());
        if metadata.is_dir() {
            for entry in std::fs::read_dir(&path)? {
                discovered = discovered.checked_add(1).ok_or("inventory count overflow")?;
                check!(discovered <= MAX_FILES);
                pending.push(entry?.path());
            }
        } else {
            check!(metadata.is_file());
            total = total.checked_add(metadata.len()).ok_or("inventory bytes overflow")?;
            check!(total <= MAX_BYTES);
            output.insert(path.strip_prefix(root)?.to_path_buf(), fingerprint(&path, metadata.len())?);
        }
    }
    check!(pending.is_empty());
    Ok(output)
}

fn fingerprint(path: &Path, expected: u64) -> TestResult<(u64, String)> {
    use std::io::Read;
    let mut file = std::fs::File::open(path)?;
    let mut buffer = [0u8; CHUNK];
    let mut hash = Sha256::new();
    let mut read = 0u64;
    for _ in 0..8193 {
        let count = file.read(&mut buffer)?;
        if count == 0 {
            check!(eq; read, expected);
            return Ok((read, format!("{:x}", hash.finalize())));
        }
        read = read.checked_add(u64::try_from(count)?).ok_or("read bytes overflow")?;
        check!(read <= expected && read <= MAX_BYTES);
        hash.update(buffer.get(..count).ok_or("bounded read slice")?);
    }
    Err("closed fixture hashing exceeds work bound".into())
}
