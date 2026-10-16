use super::*;

#[test]
fn genuine_provider_packet_reopens_exact_sources_without_unknown_school_birth() -> TestResult {
    check!(eq; hash(PROFILE), "b11d89fd95b7e08e5a90d1cbc1e51174cdb4a5f1bdf3833b980f4256ae1b266c");
    check!(eq; hash(RESULTS), "3a8c7b8c30d42117c784c8401acebd07d4cd804ea58079d244d29682b979393d");
    let (directory, store) = fresh_store()?;
    let before = protected_state(&store)?;
    let protected = non_observation_digest(&store)?;
    let packet = provider_packet(RESULTS)?;
    let reference = packet.reference()?;
    let payload = serde_json::to_value(&packet)?;
    let recording = Recording::new();
    check!(eq; record_provider_target_intake(&store, &recording, &packet, school_budget())?,
        ProviderTargetIntakeDispositionV1::RetainedOnly(ProviderTargetWithheldV1::SchoolOwnershipUnresolved));
    check!(eq; protected_state(&store)?, before);
    let recorded = recording.drain();
    check!(eq; recorded.journal, expected_journal(RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
        reference.as_str(), payload.clone()).journal);
    let expected_rows = retained_observations(&recorded)?;
    let prepared = recorded.prepare("recorded-provider-reopen")?;
    let first = prepared.apply(&store)?;
    check!(eq; first.appended(), 3);
    check!(eq; non_observation_digest(&store)?, protected);
    no_public_birth(&store)?;
    drop(store);
    let store = reopen(&directory)?;
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let reopened = read_recorded_provider_target(&snapshot, &reference, &mut grant)?
        .ok_or("provider holder")?;
    exact_root(
        &snapshot,
        reopened.root(),
        RetainedCaptureRootSchemaV1::ProviderTargetPacketV1,
        RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
        reference.as_str(),
        &payload,
    )?;
    check!(ne; reopened.root().payload.digest.as_str(), reference.as_str());
    exact_capture(&payload, Some("profile"), PROFILE)?;
    exact_capture(&payload, Some("linked_results"), RESULTS)?;
    let checked = reopened.checked();
    check!(eq; checked.reference(), &reference);
    check!(eq; checked.profile().person().id, "7223777");
    check!(eq; checked.profile().school().id, "12439");
    check!(eq; checked.profile().person_gender().raw(), Some("Female"));
    check!(eq; checked.profile().competition_category(), Gender::Unknown);
    check!(
        matches!(checked.categories(), CompetitionCategoryEvidenceV1::Agreed {
        category: Gender::Boys, rows } if rows.len() == 2)
    );
    let mut rows: Vec<census_domain::model::SourceObservation> = Vec::new();
    snapshot.for_each_observation(Table::SourceObservations, |row| {
        rows.push(row);
        Ok(())
    })?;
    let values = rows
        .iter()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()?;
    let mut actual_rows = values
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?;
    actual_rows.sort();
    check!(eq; actual_rows, expected_rows);
    let installed = protected_state(&store)?;
    let repeated = prepared.apply(&store)?;
    check!(eq; repeated.appended(), 0);
    check!(eq; repeated.receipt(), first.receipt());
    check!(eq; protected_state(&store)?, installed);
    check!(eq; non_observation_digest(&store)?, protected);
    no_public_birth(&store)?;
    Ok(())
}

fn retained_observations(recorded: &Recorded) -> TestResult<Vec<String>> {
    check!(eq; recorded.rows.len(), 1);
    check!(eq; recorded.effects.len(), 0);
    let batch = recorded.rows.first().ok_or("provider observations")?;
    check!(eq; batch.table, Table::SourceObservations);
    check!(eq; batch.rows.len(), 3);
    let mut values = batch
        .rows
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?;
    values.sort();
    Ok(values)
}

#[test]
fn unavailable_and_conflicted_category_mutations_remain_retained_without_birth() -> TestResult {
    let source = std::str::from_utf8(RESULTS)?;
    for (token, reason) in [
        (
            "unknown",
            ProviderTargetWithheldV1::CompetitionCategoryUnavailable,
        ),
        ("f", ProviderTargetWithheldV1::CompetitionCategoryConflicted),
    ] {
        let changed = source.replacen(
            "data-search=\"Zacchaeus Brocks\" data-gender=\"m\"",
            &format!("data-search=\"Zacchaeus Brocks\" data-gender=\"{token}\""),
            1,
        );
        check!(ne; changed.as_bytes(), RESULTS);
        let (directory, store) = fresh_store()?;
        let protected = non_observation_digest(&store)?;
        let packet = provider_packet(changed.as_bytes())?;
        let reference = packet.reference()?;
        let payload = serde_json::to_value(&packet)?;
        let recording = Recording::new();
        check!(eq; record_provider_target_intake(&store, &recording, &packet, school_budget())?,
            ProviderTargetIntakeDispositionV1::RetainedOnly(reason));
        let recorded = recording.drain();
        check!(eq; retained_observations(&recorded)?.len(), 3);
        check!(eq; recorded.journal, expected_journal(RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
            reference.as_str(), payload.clone()).journal);
        check!(eq; recorded.prepare("retained-category-mutation")?.apply(&store)?.appended(), 3);
        drop(store);
        let store = reopen(&directory)?;
        let snapshot = store.snapshot();
        let mut grant = AttachmentReplayGrantV1::new();
        let reopened = read_recorded_provider_target(&snapshot, &reference, &mut grant)?
            .ok_or("retained provider")?;
        exact_root(
            &snapshot,
            reopened.root(),
            RetainedCaptureRootSchemaV1::ProviderTargetPacketV1,
            RetainedCaptureRootSchemaV1::ProviderTargetPacketV1.phase(),
            reference.as_str(),
            &payload,
        )?;
        exact_capture(&payload, Some("linked_results"), changed.as_bytes())?;
        match token {
            "unknown" => check!(matches!(reopened.checked().categories(),
                CompetitionCategoryEvidenceV1::Unavailable { rows } if rows.len() == 2)),
            "f" => check!(matches!(reopened.checked().categories(),
                CompetitionCategoryEvidenceV1::Conflicted { rows } if rows.len() == 2)),
            _ => return Err("unlisted category test case".into()),
        }
        check!(eq; non_observation_digest(&store)?, protected);
        no_public_birth(&store)?;
    }
    Ok(())
}
