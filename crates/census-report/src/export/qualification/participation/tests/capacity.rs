use super::*;
use crate::report::ReportError;

#[tokio::test]
async fn real_v4_and_attachment_replay_preserve_decisions_and_refuse_missing_attachment(
) -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let live = fixture.read(&dataset, &snapshot, &fixture.profile)?;
    let wire = serde_json::to_value(&live.replay)?;
    check!(eq; wire["version"], json!(4));
    let attachment = wire["attachment"]["object"]
        .as_str()
        .ok_or("attachment object")?;
    let directory = fixture.objects();
    let raw = crate::export::proof_objects::read(&directory, attachment, 32_768)?;
    let attachment_descriptor: Value = serde_json::from_slice(&raw)?;
    check!(eq; attachment_descriptor["version"], json!(3));
    check!(attachment_descriptor["bytes"].as_u64().ok_or("bytes")? <= 536_870_912);
    check!(
        attachment_descriptor["chunks"]
            .as_array()
            .ok_or("chunks")?
            .len()
            <= 128
    );
    let descriptor = live.replay.install(&directory)?;
    let fingerprints = live.replay.fingerprints();
    let affiliations =
        crate::export::qualification::current_affiliation::PinnedCurrentAffiliations::read(
            &dataset, &snapshot,
        )?;
    let replay = || {
        PinnedParticipation::replay(
            &dataset,
            &fixture.profile,
            &directory,
            &descriptor,
            &fingerprints,
            affiliations.roster_context(),
        )
    };
    let cold = replay()?;
    check!(eq; serde_json::to_value(&cold.replay)?, wire);
    check!(eq; row(live, &fixture.performance.id)?.into_parts(), row(cold, &fixture.performance.id)?.into_parts());
    let path = directory.join(attachment);
    std::fs::remove_file(&path)?;
    match replay().err().ok_or("missing attachment accepted")? {
        ReportError::Io {
            path: actual,
            source,
        } => {
            check!(eq; actual, path);
            check!(eq; source.kind(), std::io::ErrorKind::NotFound);
        }
        other => return Err(format!("wrong attachment refusal: {other:?}").into()),
    }
    std::fs::write(&path, vec![b'x'; raw.len()])?;
    match replay().err().ok_or("altered attachment accepted")? {
        ReportError::ArtifactCorrupt {
            path: actual,
            detail,
        } => {
            check!(eq; actual, path);
            check!(eq; detail, "the frozen object does not hash to its name");
        }
        other => return Err(format!("wrong altered attachment refusal: {other:?}").into()),
    }
    std::fs::write(&path, raw)?;
    check!(eq; serde_json::to_value(&replay()?.replay)?, wire);
    Ok(())
}

#[tokio::test]
async fn old_related_physical_conflict_survives_replacement_and_frozen_replay() -> TestResult {
    for reverse in [false, true] {
        historical_conflict_roundtrip(reverse).await?;
    }
    Ok(())
}

async fn historical_conflict_roundtrip(reverse: bool) -> TestResult {
    let fixture = Fixture::prepare(Some("High School")).await?;
    let original = fixture
        .store
        .scan::<census_domain::model::CanonicalEvent>(Table::Events)?
        .into_iter()
        .find(|event| event.id == fixture.performance.event)
        .ok_or("event")?;
    let mut historical = original.clone();
    let evidence = Evidence::derived(
        SourceRef::id("capacity-historical-event"),
        "2026-10-22",
        "synthetic retained contradiction",
    );
    historical.evidence.push(evidence.clone());
    historical
        .retained_conflicts
        .push(census_domain::model::RetainedConflict::new(
            "synthetic",
            historical.id.as_str(),
            "historical event",
            "retained contradiction",
        ));
    let versions = if reverse {
        [&original, &historical]
    } else {
        [&historical, &original]
    };
    for version in versions {
        fixture.store.append(Table::Events, version)?;
    }
    let snapshot = fixture.store.snapshot();
    let dataset = fixture.dataset(&snapshot)?;
    let live = fixture.read(&dataset, &snapshot, &fixture.profile)?;
    let directory = fixture.objects();
    let descriptor = live.replay.install(&directory)?;
    let affiliations =
        crate::export::qualification::current_affiliation::PinnedCurrentAffiliations::read(
            &dataset, &snapshot,
        )?;
    let cold = PinnedParticipation::replay(
        &dataset,
        &fixture.profile,
        &directory,
        &descriptor,
        &live.replay.fingerprints(),
        affiliations.roster_context(),
    )?;
    let actual = row(live, &fixture.performance.id)?.into_parts();
    check!(eq; actual.0, CandidateOutcome::Unresolved);
    check!(actual
        .3
        .contains(&ParticipationDiagnostic::CanonicalConflict));
    check!(actual.2.contains(&format!(
        "canonical_evidence:{}",
        serialized_digest(&evidence)?
    )));
    check!(eq; row(cold, &fixture.performance.id)?.into_parts(), actual);
    Ok(())
}
