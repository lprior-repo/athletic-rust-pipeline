use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use census_domain::evidence_subject_attachment::{
    AttachmentDigestV1, AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
    EvidenceSubjectAttachmentError,
};
use census_store::{
    RetainedCaptureLeafKindV1 as Kind, RetainedCaptureLeafV1 as Leaf, RetainedCaptureRootReadV1,
    RetainedCaptureRootSchemaV1 as Schema, RetainedCaptureRootV1, RetainedCaptureSlotV1 as Slot,
};
use serde::Deserialize;
use serde_json::value::RawValue;

#[derive(Deserialize)]
struct Captures<'a> {
    #[serde(borrow)]
    directory: &'a RawValue,
    #[serde(borrow)]
    roster: &'a RawValue,
}

fn reference(bytes: &[u8]) -> TestResult<AttachmentObjectReferenceV1> {
    Ok(AttachmentObjectReferenceV1 {
        digest: AttachmentDigestV1::parse(&crate::net::cache::content_digest(bytes))?,
        bytes: u64::try_from(bytes.len())?,
    })
}

pub(super) fn verify_root(
    store: &Store,
    packet: &PublicSchoolIntakePacketV1,
    bytes: &[u8],
) -> TestResult<RetainedCaptureRootV1> {
    let mut grant = AttachmentReplayGrantV1::new();
    let view = store.snapshot();
    let root = view
        .read_retained_capture_root_v1(
            Schema::ProviderSchoolObservedPacketV1,
            packet.reference().as_str(),
            &mut grant,
        )?
        .ok_or("minted observed school root missing")?;
    check!(eq; root.payload().get().as_bytes(), bytes);
    check!(eq; root.root().payload, reference(bytes)?);
    verify_leaves(&view, &root, &mut grant)?;
    verify_bodies(root.payload().get().as_bytes())?;
    Ok(root.root().clone())
}

fn verify_leaves(
    view: &census_store::StoreSnapshot<'_>,
    root: &RetainedCaptureRootReadV1,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult {
    let captures: Captures<'_> = serde_json::from_str(root.payload().get())?;
    for (slot, literal, original) in [
        (Slot::ProviderSchoolDirectory, captures.directory, INDEX),
        (Slot::ProviderSchoolRoster, captures.roster, ROSTER),
    ] {
        let mut leaf = Leaf {
            version: 1,
            root: root.root().clone(),
            slot,
            kind: Kind::CaptureJson,
            object: reference(literal.get().as_bytes())?,
        };
        view.verify_retained_capture_leaf_v1(&leaf, grant)?;
        leaf.kind = Kind::OriginalBody;
        leaf.object = reference(original)?;
        view.verify_retained_capture_leaf_v1(&leaf, grant)?;
    }
    Ok(())
}

fn verify_bodies(bytes: &[u8]) -> TestResult {
    let decoded = PublicSchoolIntakePacketV1::decode(bytes)?;
    check!(eq; decoded.directory().body(), INDEX);
    check!(eq; decoded.roster().body(), ROSTER);
    check!(eq; decoded.directory().work().request().original_url(), "https://mi.milesplit.com/teams");
    check!(eq; decoded.roster().work().request().original_url(), format!("{ORIGIN}{PATH}"));
    check!(eq; crate::net::cache::content_digest(INDEX), "39606bf8e07f0170138bb586ae8a762903bb820b515520e89e34745f6c732351");
    check!(eq; crate::net::cache::content_digest(ROSTER), "246374a87ba6d56802e58e54e4a1edcc5d1eee52106dedb24df185cbd037e178");
    Ok(())
}

pub(super) fn refuse_tampered_packet(packet: &PublicSchoolIntakePacketV1) -> TestResult {
    let bytes = packet.encode()?;
    for (pointer, replacement, expected) in [
        (
            "/directory/body",
            STANDARD.encode(b"changed original"),
            "public school intake original body differs",
        ),
        (
            "/roster/terminal_receipt/digest",
            "0".repeat(64),
            "public school intake terminal proof differs",
        ),
    ] {
        let mut wire: serde_json::Value = serde_json::from_slice(&bytes)?;
        *wire
            .pointer_mut(pointer)
            .ok_or("actual immutable packet field")? = replacement.into();
        let mut changed = Vec::new();
        census_domain::model::write_serialized(&mut changed, &wire)?;
        check!(matches!(PublicSchoolIntakePacketV1::decode(&changed),
            Err(StoreError::Refused { detail }) if detail == expected));
    }
    Ok(())
}

pub(super) fn refuse_changed_leaf(
    store: &Store,
    packet: &PublicSchoolIntakePacketV1,
) -> TestResult {
    let view = store.snapshot();
    let root = view
        .read_retained_capture_root_v1(
            Schema::ProviderSchoolObservedPacketV1,
            packet.reference().as_str(),
            &mut AttachmentReplayGrantV1::new(),
        )?
        .ok_or("observed school root")?;
    for (kind, expected) in [
        (Kind::CaptureJson, "external leaf reference differs"),
        (
            Kind::OriginalBody,
            "external body reference differs from capture",
        ),
    ] {
        let leaf = Leaf {
            version: 1,
            root: root.root().clone(),
            slot: Slot::ProviderSchoolRoster,
            kind,
            object: reference(b"changed leaf")?,
        };
        let mut grant = AttachmentReplayGrantV1::new();
        let refusal = view
            .verify_retained_capture_leaf_v1(&leaf, &mut grant)
            .err()
            .ok_or("changed external leaf was accepted")?;
        check!(matches!(refusal, StoreError::EvidenceSubjectAttachment(
            census_store::EvidenceSubjectAttachmentStoreError::Corrupt(detail)
        ) if detail == expected));
        check!(matches!(
            grant.ensure_available(),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    Ok(())
}
