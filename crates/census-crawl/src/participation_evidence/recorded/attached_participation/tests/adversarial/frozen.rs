use super::*;
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentObjectReferenceV1, AttachmentSubjectRoleV1,
};
use census_store::{
    replay_frozen_evidence_subject_attachment_references_v1 as replay,
    DiscoveredAttachmentCaptureV1, FrozenEvidenceSubjectAttachmentBundleV1 as Bundle,
    RetainedCaptureLeafV1,
};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use sha2::{Digest, Sha256};

pub(super) fn verify(store: &Store) -> TestResult {
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted authority")?;
    let mut grant = Grant::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_with_accepted_v1(
        &accepted, &mut grant,
    )?;
    let metadata =
        current.discover_active_subjects_v1(AttachmentSubjectRoleV1::AthleteStatus, &mut grant)?;
    let leaf = metadata
        .subjects()
        .iter()
        .find_map(|item| match item.capture() {
            DiscoveredAttachmentCaptureV1::Retained(leaf) => Some(leaf.clone()),
            _ => None,
        })
        .ok_or("actual retained status leaf")?;
    snapshot.verify_retained_capture_leaf_v1(&leaf, &mut grant)?;
    let bundle = current.freeze_closure(&mut grant)?;
    let mut frozen_grant = Grant::new();
    let frozen = replay(&bundle, &accepted, &mut frozen_grant)?;
    frozen
        .source()
        .verify_retained_capture_leaf_v1(&leaf, &mut frozen_grant)?;
    for wrong in [wrong_root(&leaf)?, wrong_leaf(&leaf)?] {
        let mut live_grant = Grant::new();
        check!(snapshot
            .verify_retained_capture_leaf_v1(&wrong, &mut live_grant)
            .is_err());
        check!(live_grant.ensure_available().is_err());
        let mut replay_grant = Grant::new();
        let owner = replay(&bundle, &accepted, &mut replay_grant)?;
        check!(owner
            .source()
            .verify_retained_capture_leaf_v1(&wrong, &mut replay_grant)
            .is_err());
        check!(replay_grant.ensure_available().is_err());
    }
    let original = bundle.bytes().to_vec();
    for change in [HeaderChange::Head, HeaderChange::Snapshot] {
        let mut wire: Archive = serde_json::from_slice(&original)?;
        check!(eq; serde_json::to_vec(&wire)?, original);
        match change {
            HeaderChange::Head => wire.public_head.digest = different(&wire.public_head.digest)?,
            HeaderChange::Snapshot => {
                wire.snapshot_sequence = wire
                    .snapshot_sequence
                    .checked_add(1)
                    .ok_or("snapshot overflow")?
            }
        }
        let bytes = serde_json::to_vec(&wire)?;
        let reference = AttachmentObjectReferenceV1 {
            digest: AttachmentDigestV1::parse(&format!("{:x}", Sha256::digest(&bytes)))?,
            bytes: u64::try_from(bytes.len())?,
        };
        let mut altered_grant = Grant::new();
        let altered = Bundle::from_owned_bytes(bytes, reference, &mut altered_grant)?;
        check!(matches!(
            replay(&altered, &accepted, &mut altered_grant),
            Err(census_store::StoreError::EvidenceSubjectAttachment(
                census_store::EvidenceSubjectAttachmentStoreError::TargetMoved
            ))
        ));
        check!(altered_grant.ensure_available().is_err());
    }
    check!(eq; bundle.bytes(), original);
    Ok(())
}

fn wrong_root(leaf: &RetainedCaptureLeafV1) -> TestResult<RetainedCaptureLeafV1> {
    let mut wrong = leaf.clone();
    wrong.root.payload.digest =
        AttachmentDigestV1::parse(&different(wrong.root.payload.digest.as_str())?)?;
    Ok(wrong)
}

fn wrong_leaf(leaf: &RetainedCaptureLeafV1) -> TestResult<RetainedCaptureLeafV1> {
    let mut wrong = leaf.clone();
    wrong.object.digest = AttachmentDigestV1::parse(&different(wrong.object.digest.as_str())?)?;
    Ok(wrong)
}

fn different(value: &str) -> TestResult<String> {
    check!(eq; value.len(), 64);
    let mut bytes = value.as_bytes().to_vec();
    let first = bytes.first_mut().ok_or("digest byte")?;
    *first = if *first == b'0' { b'1' } else { b'0' };
    Ok(String::from_utf8(bytes)?)
}

enum HeaderChange {
    Head,
    Snapshot,
}

#[derive(Serialize, Deserialize)]
struct Archive {
    schema: String,
    store: String,
    snapshot_sequence: u64,
    public_head: Head,
    inventory: Box<RawValue>,
    prefixes: Vec<Box<RawValue>>,
}

#[derive(Serialize, Deserialize)]
struct Head {
    sequence: u64,
    digest: String,
}
