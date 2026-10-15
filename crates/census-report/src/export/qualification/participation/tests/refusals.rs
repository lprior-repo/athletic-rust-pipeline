use super::*;

#[tokio::test]
async fn foreign_snapshot_profile_policy_and_mutated_dataset_fail_closed() -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let view = fixture.store.snapshot();
    let mut dataset = fixture.dataset(&view)?;
    let foreign = Fixture::prepare(Some("High School")).await?;
    assert!(fixture
        .read(&dataset, &foreign.store.snapshot(), &fixture.profile)
        .is_err());
    let unknown = profile::profile(census_domain::model::ContractRefV1::new(
        1,
        2,
        &"b".repeat(64),
    )?)?;
    assert!(fixture.read(&dataset, &view, &unknown).is_err());
    dataset.performances.first_mut().ok_or("performance")?.date = "2026-01-01".into();
    assert!(fixture.read(&dataset, &view, &fixture.profile).is_err());
    Ok(())
}

#[tokio::test]
async fn missing_source_archive_is_unresolved_with_attempted_proof_references() -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let mut performance = fixture.performance.clone();
    let metadata = performance
        .evidence
        .iter_mut()
        .find(|evidence| {
            evidence
                .note
                .as_ref()
                .is_some_and(|note| note.contains("raw_metadata_capture"))
        })
        .ok_or("metadata evidence")?;
    let mut value: Value = serde_json::from_str(metadata.note.as_deref().ok_or("note")?)?;
    let missing = format!("metadata/{}/manifest", "b".repeat(64));
    value["raw_metadata_capture"]["archive_manifest"] = json!(missing);
    metadata.note = Some(value.to_string());
    fixture.store.append(Table::Performances, &performance)?;
    let (outcome, _, refs, gaps) = row(fixture.assess()?, &performance.id)?.into_parts();
    assert_eq!(outcome, CandidateOutcome::Unresolved);
    assert!(gaps.contains(&ParticipationDiagnostic::SourceUnavailable));
    assert!(refs.iter().any(|reference| reference.ends_with(&missing)));
    Ok(())
}

#[tokio::test]
async fn pinned_view_ignores_later_supplement_while_next_view_preserves_it() -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let view = fixture.store.snapshot();
    let dataset = fixture.dataset(&view)?;
    let mut athlete = fixture
        .store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .into_iter()
        .find(|row| row.id == fixture.performance.athlete)
        .ok_or("athlete")?;
    athlete.evidence.push(Evidence::derived(
        SourceRef::id("synthetic-status"),
        "2026-10-01",
        "additional unverified student status",
    ));
    fixture.store.append(Table::Athletes, &athlete)?;
    let pinned = fixture.read(&dataset, &view, &fixture.profile)?;
    let (outcome, reasons, _, gaps) = row(pinned, &fixture.performance.id)?.into_parts();
    assert_eq!(outcome, CandidateOutcome::Unresolved);
    assert_eq!(
        reasons.iter().collect::<Vec<_>>(),
        vec![
            ParticipationReason::HighSchoolStatusMissing,
            ParticipationReason::SummerBoundaryUnresolved,
        ]
    );
    assert!(!gaps.contains(&ParticipationDiagnostic::UnverifiedAdditionalEvidence));
    let (outcome, _, _, gaps) = row(fixture.assess()?, &fixture.performance.id)?.into_parts();
    assert_eq!(outcome, CandidateOutcome::Unresolved);
    assert!(gaps.contains(&ParticipationDiagnostic::UnverifiedAdditionalEvidence));
    Ok(())
}

#[tokio::test]
async fn copied_producer_note_with_changed_outer_owner_cannot_close_evidence() -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let mut athlete = fixture
        .store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .into_iter()
        .find(|row| row.id == fixture.performance.athlete)
        .ok_or("athlete")?;
    let mut borrowed = athlete.evidence.first().ok_or("evidence")?.clone();
    borrowed.source.id = "unverified-other-provider".into();
    athlete.evidence.push(borrowed);
    fixture.store.append(Table::Athletes, &athlete)?;
    let (outcome, _, _, gaps) = row(fixture.assess()?, &fixture.performance.id)?.into_parts();
    assert_eq!(outcome, CandidateOutcome::Unresolved);
    assert!(gaps.contains(&ParticipationDiagnostic::UnverifiedAdditionalEvidence));
    Ok(())
}

#[tokio::test]
async fn recorded_source_journal_replays_through_the_same_owned_and_raw_readers() -> TestResult {
    use census_crawl::milesplit::{
        read_retained_owned_result_from, read_retained_result_metadata_from, CaptureReadBudget,
        FrozenRetainedJournal,
    };
    use sha2::{Digest, Sha256};
    let fixture = Fixture::prepare(Some("High School")).await?;
    let token = fixture.assess()?;
    let bytes = token.replay.journal.encode()?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    let frozen = FrozenRetainedJournal::from_descriptor(
        &digest,
        &token.replay.store_identity,
        token.replay.snapshot_sequence,
        &bytes,
    )?;
    let evidence: Vec<_> = fixture
        .performance
        .evidence
        .iter()
        .map(|evidence| Ok((evidence, note::value(evidence)?.ok_or("source note")?)))
        .collect::<TestResult<_>>()?;
    let (row, row_note) = evidence
        .iter()
        .find(|(_, value)| note::owned(value))
        .ok_or("owned note")?;
    let (meta, meta_note) = evidence
        .iter()
        .find(|(_, value)| note::metadata(value))
        .ok_or("metadata note")?;
    let notes = note::SourceNotes::read(row, row_note, meta, meta_note)?;
    let budget = CaptureReadBudget::new(32 * 1024 * 1024)?;
    let owned = read_retained_owned_result_from(&frozen, &notes.owned, &notes.locator, budget)?;
    let source =
        read_retained_result_metadata_from(&frozen, &notes.reference, &notes.raw, &owned, budget)?;
    assert_eq!(source.result().row().result_id, 201782263);
    assert_eq!(
        source.division(),
        census_crawl::milesplit::RetainedResultDivision::HighSchool
    );
    assert_eq!(source.snapshot_sequence(), token.replay.snapshot_sequence);
    assert_eq!(
        source.store_identity().as_str(),
        token.replay.store_identity
    );
    Ok(())
}

#[tokio::test]
async fn immutable_replay_objects_round_trip_and_refuse_foreign_descriptor() -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let token = fixture.assess()?;
    let directory = tempfile::tempdir()?;
    let descriptor = token.replay.install(directory.path())?;
    let replay = ParticipationReplayV1::load(directory.path(), &descriptor)?;
    assert_eq!(
        serde_json::to_value(&replay)?,
        serde_json::to_value(&token.replay)?
    );
    assert!(ParticipationReplayV1::load(directory.path(), &"f".repeat(64)).is_err());
    Ok(())
}

#[tokio::test]
async fn complete_frozen_participation_replays_identical_decisions_without_store_reads(
) -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let live = fixture.read(&dataset, &snapshot, &fixture.profile)?;
    let directory = fixture.objects();
    let descriptor = live.replay.install(&directory)?;
    let fingerprints = live.replay.fingerprints();
    let affiliations =
        crate::export::qualification::current_affiliation::PinnedCurrentAffiliations::read(
            &dataset, &snapshot,
        )?;
    let frozen = PinnedParticipation::replay(
        &dataset,
        &fixture.profile,
        &directory,
        &descriptor,
        &fingerprints,
        affiliations.roster_context(),
    )?;
    let live = row(live, &fixture.performance.id)?.into_parts();
    let frozen = row(frozen, &fixture.performance.id)?.into_parts();
    assert_eq!(live, frozen);
    let mut foreign = fingerprints;
    foreign.insert("performances".into(), "a".repeat(64));
    assert!(PinnedParticipation::replay(
        &dataset,
        &fixture.profile,
        &directory,
        &descriptor,
        &foreign,
        affiliations.roster_context(),
    )
    .is_err());
    Ok(())
}

#[tokio::test]
async fn unverified_supplement_prevents_a_definitive_source_exclusion() -> TestResult {
    let fixture = Fixture::prepare(Some("Collegiate")).await?;
    let mut athlete = fixture
        .store
        .scan::<CanonicalAthlete>(Table::Athletes)?
        .into_iter()
        .find(|row| row.id == fixture.performance.athlete)
        .ok_or("athlete")?;
    athlete.evidence.push(Evidence::derived(
        SourceRef::id("synthetic-conflicting-status"),
        "2026-10-01",
        "unverified contradictory high-school status",
    ));
    fixture.store.append(Table::Athletes, &athlete)?;
    let (outcome, reasons, refs, gaps) =
        row(fixture.assess()?, &fixture.performance.id)?.into_parts();
    assert_eq!(outcome, CandidateOutcome::Unresolved);
    assert!(reasons.contains(ParticipationReason::CollegiateParticipation));
    assert!(!refs.is_empty());
    assert!(gaps.contains(&ParticipationDiagnostic::UnverifiedAdditionalEvidence));
    Ok(())
}

#[tokio::test]
async fn authentic_source_note_cannot_bind_missing_or_foreign_canonical_context() -> TestResult {
    for field in ["team", "event", "meet"] {
        let fixture = Fixture::prepare(Some("High School")).await?;
        let mut value = serde_json::to_value(&fixture.performance)?;
        value[field] = json!(format!("synthetic-foreign-{field}"));
        let changed: CanonicalPerformance = serde_json::from_value(value)?;
        fixture.store.append(Table::Performances, &changed)?;
        let (outcome, _, _, gaps) = row(fixture.assess()?, &changed.id)?.into_parts();
        assert_eq!(outcome, CandidateOutcome::Unresolved);
        assert!(
            gaps.contains(&ParticipationDiagnostic::SourceBindingMismatch)
                || gaps.contains(&ParticipationDiagnostic::CanonicalConflict)
        );
    }
    Ok(())
}

#[tokio::test]
async fn historical_team_link_keeps_supplementary_evidence_in_closure() -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let mut team = fixture
        .store
        .scan::<census_domain::model::CanonicalTeam>(Table::Teams)?
        .into_iter()
        .find(|team| team.id == fixture.performance.team)
        .ok_or("team")?;
    team.id = census_domain::model::CanonicalTeam::mint(
        &team.school,
        team.sport,
        census_domain::model::Gender::Boys,
        team.school_year,
    );
    team.evidence.push(Evidence::derived(
        SourceRef::id("synthetic-historical-window"),
        "2026-10-01",
        "unverified status on superseded team link",
    ));
    fixture.store.append(Table::Teams, &team)?;
    let mut historical = fixture.performance.clone();
    historical.team = team.id;
    fixture.store.append(Table::Performances, &historical)?;
    fixture
        .store
        .append(Table::Performances, &fixture.performance)?;
    let (outcome, _, refs, gaps) = row(fixture.assess()?, &fixture.performance.id)?.into_parts();
    assert_eq!(outcome, CandidateOutcome::Unresolved);
    assert!(gaps.contains(&ParticipationDiagnostic::UnverifiedAdditionalEvidence));
    let expected = format!(
        "canonical_evidence:{}",
        serialized_digest(team.evidence.last().ok_or("supplement")?)?
    );
    assert!(refs.contains(&expected));
    Ok(())
}
