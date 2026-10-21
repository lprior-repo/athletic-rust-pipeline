use super::*;
use census_store::EvidenceSubjectAttachmentStoreError;

#[test]
fn duplicate_missing_and_rejected_directory_rows_preserve_existing_school_intake() -> TestResult {
    let (_directory, store) = fresh_store()?;
    let original = packet(DIRECTORY)?;
    let recording = Recording::new();
    record_provider_school_intake(&store, &recording, &original)?;
    let expected = recording.drain();
    let retained = install_for_refusal(
        expected.clone(),
        &store,
        "genuine-school-before-directory-refusals",
    )?;
    record_provider_school_intake(&store, &recording, &original)?;
    let usage = recording.usage();
    let before = protected_state(&store)?;
    let text = std::str::from_utf8(DIRECTORY)?;
    let row = text
        .get(91718..91989)
        .ok_or("literal original directory row")?;
    let invalid =
        "<tr><td><a href=\"https://mi.milesplit.com/teams/999-invalid\">Bad</a></td></tr>";
    for (replacement, reason) in [
        (
            format!("{row}{row}"),
            "provider school directory owner is ambiguous",
        ),
        (String::new(), "provider school directory owner absent"),
        (
            format!("{row}{invalid}"),
            "provider school directory interpretation is incomplete",
        ),
    ] {
        let changed = text.replacen(row, &replacement, 1);
        check!(ne; changed.as_bytes(), DIRECTORY);
        let changed_packet = packet(changed.as_bytes())?;
        check!(
            matches!(record_provider_school_intake(&store, &recording, &changed_packet),
            Err(CrawlError::Store(StoreError::Refused { detail })) if detail == reason)
        );
        check!(eq; recording.usage(), usage);
        require_retained_state(&store, &retained)?;
        check!(eq; protected_state(&store)?, before);
    }
    check!(eq; recording.drain(), expected);
    no_public_birth(&store)?;
    Ok(())
}

#[derive(Clone, Copy)]
enum ChangedRoot {
    TypedKey,
    BodyDigest,
    BodyBound,
    ForeignPrefix,
}

#[test]
fn changed_school_roots_refuse_with_sticky_grant_after_genuine_installation() -> TestResult {
    for change in [
        ChangedRoot::TypedKey,
        ChangedRoot::BodyDigest,
        ChangedRoot::BodyBound,
        ChangedRoot::ForeignPrefix,
    ] {
        let (directory, store) = fresh_store()?;
        let original = packet(DIRECTORY)?;
        let original_reference = original.reference()?;
        let recording = Recording::new();
        record_provider_school_intake(&store, &recording, &original)?;
        let mut recorded = recording.drain();
        let prior = install_for_refusal(
            recorded.clone(),
            &store,
            "genuine-school-before-root-change",
        )?;
        let mut lookup = original_reference.clone();
        match change {
            ChangedRoot::TypedKey => {
                lookup = ProviderSchoolPacketRefV1::parse(&"f".repeat(64))?;
                check!(ne; lookup, original_reference);
                recorded
                    .journal
                    .first_mut()
                    .ok_or("school original journal")?
                    .key = lookup.as_str().into();
            }
            ChangedRoot::BodyDigest | ChangedRoot::BodyBound => {
                let capture = recorded
                    .journal
                    .first_mut()
                    .ok_or("school original journal")?
                    .payload
                    .get_mut("roster")
                    .ok_or("original roster capture")?;
                match change {
                    ChangedRoot::BodyDigest => {
                        *capture.get_mut("body_sha256").ok_or("roster digest")? =
                            Value::String("0".repeat(64))
                    }
                    ChangedRoot::BodyBound => {
                        *capture.get_mut("body_bytes").ok_or("roster body bytes")? =
                            Value::from(2 * 1024 * 1024 + 1)
                    }
                    _ => return Err("unexpected school body mutation".into()),
                }
            }
            ChangedRoot::ForeignPrefix => {
                let mut foreign = recorded
                    .journal
                    .first()
                    .ok_or("genuine school root")?
                    .clone();
                foreign.key.push('0');
                recorded.journal.try_reserve_exact(1)?;
                recorded.journal.push(foreign);
            }
        }
        let retained = install_for_refusal(recorded, &store, "changed-school-root")?;
        check!(eq; store.receipt(&prior.receipt.operation)?, Some(prior.receipt.clone()));
        drop(store);
        let store = reopen(&directory)?;
        let before = protected_state(&store)?;
        let snapshot = store.snapshot();
        let mut grant = AttachmentReplayGrantV1::new();
        let first = read_recorded_provider_school(&snapshot, &lookup, &mut grant);
        match change {
            ChangedRoot::TypedKey => check!(matches!(first,
                Err(CrawlError::Store(StoreError::Refused { detail }))
                if detail == "recorded provider school packet differs from its typed key")),
            ChangedRoot::BodyDigest => check!(matches!(first,
                Err(CrawlError::Store(StoreError::Refused { detail }))
                if detail == "participation HTML body length or digest differs")),
            ChangedRoot::BodyBound => check!(matches!(
                first,
                Err(CrawlError::Store(StoreError::EvidenceSubjectAttachment(
                    EvidenceSubjectAttachmentStoreError::Corrupt("external capture body bounds")
                )))
            )),
            ChangedRoot::ForeignPrefix => check!(matches!(
                first,
                Err(CrawlError::Store(StoreError::EvidenceSubjectAttachment(
                    EvidenceSubjectAttachmentStoreError::Corrupt(
                        "external root prefix has a foreign root"
                    )
                )))
            )),
        }
        require_retained_state(&store, &retained)?;
        if matches!(change, ChangedRoot::TypedKey | ChangedRoot::ForeignPrefix) {
            require_retained_state(&store, &prior)?;
        }
        check!(eq; protected_state(&store)?, before);
        check!(matches!(
            read_recorded_provider_school(&snapshot, &original_reference, &mut grant),
            Err(CrawlError::Store(StoreError::EvidenceSubjectAttachment(
                EvidenceSubjectAttachmentStoreError::Domain(
                    EvidenceSubjectAttachmentError::Poisoned
                )
            )))
        ));
        require_retained_state(&store, &retained)?;
        if matches!(change, ChangedRoot::TypedKey | ChangedRoot::ForeignPrefix) {
            require_retained_state(&store, &prior)?;
        }
        check!(eq; store.receipt(&prior.receipt.operation)?, Some(prior.receipt.clone()));
        check!(eq; protected_state(&store)?, before);
        no_public_birth(&store)?;
    }
    Ok(())
}
