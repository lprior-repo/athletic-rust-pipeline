use super::refusals::{genuine_recorded, read_present, require_poisoned, Family, FAMILIES};
use super::*;
use census_store::EvidenceSubjectAttachmentStoreError;

#[derive(Clone, Copy)]
enum ShapeTarget {
    Packet,
    Capture,
    Provenance,
}

fn sequence_shaped(payload: &mut Value, family: Family, target: ShapeTarget) -> TestResult {
    let selected = match target {
        ShapeTarget::Packet => payload,
        ShapeTarget::Capture | ShapeTarget::Provenance => {
            let capture = match family {
                Family::Provider => payload.get_mut("profile").ok_or("profile capture")?,
                Family::Html => payload,
                Family::Participation => payload.get_mut("status").ok_or("status capture")?,
            };
            match target {
                ShapeTarget::Provenance => capture.get_mut("provenance").ok_or("provenance")?,
                ShapeTarget::Capture => capture,
                ShapeTarget::Packet => return Err("packet selected twice".into()),
            }
        }
    };
    let fields = selected
        .as_object()
        .ok_or("original object")?
        .values()
        .cloned()
        .collect();
    *selected = Value::Array(fields);
    Ok(())
}

#[test]
fn sequence_shaped_packet_capture_and_provenance_roots_refuse_without_mutation() -> TestResult {
    for (family, target) in [
        (Family::Provider, ShapeTarget::Packet),
        (Family::Html, ShapeTarget::Packet),
        (Family::Participation, ShapeTarget::Packet),
        (Family::Provider, ShapeTarget::Capture),
        (Family::Participation, ShapeTarget::Capture),
        (Family::Provider, ShapeTarget::Provenance),
        (Family::Html, ShapeTarget::Provenance),
        (Family::Participation, ShapeTarget::Provenance),
    ] {
        let (directory, store) = fresh_store()?;
        let (mut recorded, reference) = genuine_recorded(&store, family)?;
        sequence_shaped(
            &mut recorded.journal.first_mut().ok_or("original root")?.payload,
            family,
            target,
        )?;
        let retained = install_for_refusal(recorded, &store, "sequence-shaped-source")?;
        drop(store);
        let store = reopen(&directory)?;
        let before = protected_state(&store)?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            read_present(&store, family, &reference, &mut grant),
            Err(CrawlError::Store(StoreError::EvidenceSubjectAttachment(
                EvidenceSubjectAttachmentStoreError::Corrupt(
                    "external schema requires a JSON object"
                )
            )))
        ));
        require_retained_state(&store, &retained)?;
        require_poisoned(read_present(&store, family, &reference, &mut grant))?;
        require_retained_state(&store, &retained)?;
        check!(eq; protected_state(&store)?, before);
        no_public_birth(&store)?;
    }
    Ok(())
}

#[test]
fn foreign_root_under_a_genuine_key_prefix_refuses_without_changing_state() -> TestResult {
    for family in FAMILIES {
        let (directory, store) = fresh_store()?;
        let (mut recorded, reference) = genuine_recorded(&store, family)?;
        let mut foreign = recorded
            .journal
            .first()
            .ok_or("genuine original root")?
            .clone();
        foreign.key.push('0');
        recorded.journal.try_reserve_exact(1)?;
        recorded.journal.push(foreign);
        let retained = install_for_refusal(recorded, &store, "foreign-original-root-prefix")?;
        drop(store);
        let store = reopen(&directory)?;
        let before = protected_state(&store)?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            read_present(&store, family, &reference, &mut grant),
            Err(CrawlError::Store(StoreError::EvidenceSubjectAttachment(
                EvidenceSubjectAttachmentStoreError::Corrupt(
                    "external root prefix has a foreign root"
                )
            )))
        ));
        require_retained_state(&store, &retained)?;
        require_poisoned(read_present(&store, family, &reference, &mut grant))?;
        require_retained_state(&store, &retained)?;
        check!(eq; protected_state(&store)?, before);
        no_public_birth(&store)?;
    }
    Ok(())
}
