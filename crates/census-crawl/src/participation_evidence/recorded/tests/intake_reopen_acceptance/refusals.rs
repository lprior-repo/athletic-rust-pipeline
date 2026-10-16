use super::*;
use census_store::EvidenceSubjectAttachmentStoreError;

#[derive(Clone, Copy)]
pub(super) enum Family {
    Provider,
    Html,
    Participation,
}

pub(super) const FAMILIES: [Family; 3] = [Family::Provider, Family::Html, Family::Participation];

pub(super) fn genuine_recorded(store: &Store, family: Family) -> TestResult<(Recorded, String)> {
    let recording = Recording::new();
    let reference = match family {
        Family::Provider => {
            let packet = provider_packet(RESULTS)?;
            check!(eq; record_provider_target_intake(store, &recording, &packet, school_budget())?,
                ProviderTargetIntakeDispositionV1::RetainedOnly(ProviderTargetWithheldV1::SchoolOwnershipUnresolved));
            packet.reference()?.as_str().to_string()
        }
        Family::Html => {
            let capture = html_capture()?;
            let document = read_retained_html_result_rows(&capture, &locators())?;
            record_retained_html_result_capture(store, &recording, &document)?;
            document.context_reference().to_string()
        }
        Family::Participation => {
            let packet = participation_packet()?;
            record_independent_participation_packet(store, &recording, &packet)?;
            packet.reference()?.as_str().to_string()
        }
    };
    Ok((recording.drain(), reference))
}

pub(super) fn read_present(
    store: &Store,
    family: Family,
    key: &str,
    grant: &mut AttachmentReplayGrantV1,
) -> crate::CrawlResult<bool> {
    let snapshot = store.snapshot();
    match family {
        Family::Provider => {
            read_recorded_provider_target(&snapshot, &SourceTargetPacketRefV1::parse(key)?, grant)
                .map(|value| value.is_some())
        }
        Family::Html => read_recorded_html_result_capture(&snapshot, key, &locators(), grant)
            .map(|value| value.is_some()),
        Family::Participation => read_recorded_independent_participation(
            &snapshot,
            &ParticipationEvidenceRefV1::parse(key)?,
            grant,
        )
        .map(|value| value.is_some()),
    }
}

#[test]
fn substituted_typed_keys_refuse_after_real_recording_installation_and_reopen() -> TestResult {
    for family in FAMILIES {
        let (directory, store) = fresh_store()?;
        let (mut recorded, original) = genuine_recorded(&store, family)?;
        let wrong = "f".repeat(64);
        check!(ne; original, wrong);
        recorded
            .journal
            .first_mut()
            .ok_or("original journal root")?
            .key = wrong.clone();
        let retained = install_for_refusal(recorded, &store, "substituted-source-key")?;
        drop(store);
        let store = reopen(&directory)?;
        let before = protected_state(&store)?;
        let expected = match family {
            Family::Provider => "recorded provider target packet differs from its typed key",
            Family::Html => "recorded HTML result context differs from its typed key",
            Family::Participation => "recorded participation packet differs from its typed key",
        };
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(read_present(&store, family, &wrong, &mut grant),
            Err(CrawlError::Store(StoreError::Refused { detail })) if detail == expected));
        require_retained_state(&store, &retained)?;
        require_poisoned(read_present(&store, family, &original, &mut grant))?;
        let mut independent_read = AttachmentReplayGrantV1::new();
        check!(eq; read_present(&store, family, &original, &mut independent_read)?, false);
        require_retained_state(&store, &retained)?;
        check!(eq; protected_state(&store)?, before);
        no_public_birth(&store)?;
    }
    Ok(())
}

mod tampering;

#[test]
fn wrong_phase_keeps_original_missing_and_refuses_cross_schema_payloads() -> TestResult {
    for family in FAMILIES {
        let (directory, store) = fresh_store()?;
        let (mut recorded, reference) = genuine_recorded(&store, family)?;
        let (foreign, schema) = match family {
            Family::Provider | Family::Html => (
                Family::Participation,
                RetainedCaptureRootSchemaV1::ParticipationPacketV1,
            ),
            Family::Participation => (
                Family::Provider,
                RetainedCaptureRootSchemaV1::ProviderTargetPacketV1,
            ),
        };
        recorded.journal.first_mut().ok_or("original root")?.phase = schema.phase().into();
        let retained = install_for_refusal(recorded, &store, "cross-schema-original")?;
        drop(store);
        let store = reopen(&directory)?;
        let before = protected_state(&store)?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(eq; read_present(&store, family, &reference, &mut grant)?, false);
        grant.ensure_available()?;
        check!(
            matches!(read_present(&store, foreign, &reference, &mut grant),
            Err(CrawlError::Store(StoreError::Decode { key, .. }))
            if key == "attachment external capture schema")
        );
        require_retained_state(&store, &retained)?;
        require_poisoned(read_present(&store, family, &reference, &mut grant))?;
        require_retained_state(&store, &retained)?;
        check!(eq; protected_state(&store)?, before);
        no_public_birth(&store)?;
    }
    Ok(())
}

#[test]
fn missing_reference_returns_absence_without_inventing_a_root_or_mutating_state() -> TestResult {
    let (_directory, store) = fresh_store()?;
    let before = protected_state(&store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    for family in FAMILIES {
        check!(eq; read_present(&store, family, &"d".repeat(64), &mut grant)?, false);
        grant.ensure_available()?;
        check!(eq; protected_state(&store)?, before);
    }
    no_public_birth(&store)?;
    Ok(())
}

#[test]
fn malformed_originals_refuse_without_changing_an_existing_recording_or_store() -> TestResult {
    let (_directory, store) = fresh_store()?;
    let packet = participation_packet()?;
    let expected = expected_journal(
        PARTICIPATION_EVIDENCE_PHASE,
        packet.reference()?.as_str(),
        packet.payload()?,
    );
    let recording = Recording::new();
    record_independent_participation_packet(&store, &recording, &packet)?;
    let usage = recording.usage();
    let before = protected_state(&store)?;
    let mut bad_provider = serde_json::to_value(provider_packet(RESULTS)?)?;
    *bad_provider
        .get_mut("profile")
        .and_then(|value| value.get_mut("body_sha256"))
        .ok_or("provider digest")? = Value::String("0".repeat(64));
    let bad_provider: SourceTargetPacketV1 = serde_json::from_value(bad_provider)?;
    check!(
        matches!(record_provider_target_intake(&store, &recording, &bad_provider, school_budget()),
        Err(CrawlError::Store(StoreError::Refused { detail }))
        if detail == "participation HTML body length or digest differs")
    );
    let mut bad_participation = packet.payload()?;
    *bad_participation
        .get_mut("schedule")
        .and_then(|value| value.get_mut("body_sha256"))
        .ok_or("schedule digest")? = Value::String("0".repeat(64));
    let bad_participation: IndependentParticipationPacketV1 =
        serde_json::from_value(bad_participation)?;
    check!(
        matches!(record_independent_participation_packet(&store, &recording, &bad_participation),
        Err(CrawlError::Store(StoreError::Refused { detail }))
        if detail == "participation HTML body length or digest differs")
    );
    check!(eq; recording.usage(), usage);
    check!(eq; recording.drain(), expected);
    check!(eq; protected_state(&store)?, before);
    no_public_birth(&store)?;
    Ok(())
}

#[test]
fn invalid_school_budget_refuses_without_partially_recording_a_valid_packet() -> TestResult {
    let (_directory, store) = fresh_store()?;
    let before = protected_state(&store)?;
    let packet = provider_packet(RESULTS)?;
    for budget in [
        ReadBudget {
            rows: 0,
            ..school_budget()
        },
        ReadBudget {
            bytes: 0,
            ..school_budget()
        },
        ReadBudget {
            record_bytes: 0,
            ..school_budget()
        },
    ] {
        let recording = Recording::new();
        check!(
            matches!(record_provider_target_intake(&store, &recording, &packet, budget),
            Err(CrawlError::Store(StoreError::Refused { detail }))
            if detail == "provider target school scan requires a nonzero budget")
        );
        check!(eq; recording.drain(), Recorded::default());
        check!(eq; protected_state(&store)?, before);
    }
    Ok(())
}

pub(super) fn require_poisoned(result: crate::CrawlResult<bool>) -> TestResult {
    check!(matches!(
        result,
        Err(CrawlError::Store(StoreError::EvidenceSubjectAttachment(
            EvidenceSubjectAttachmentStoreError::Domain(EvidenceSubjectAttachmentError::Poisoned)
        )))
    ));
    Ok(())
}
