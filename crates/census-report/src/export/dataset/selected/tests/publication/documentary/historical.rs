use super::*;
use crate::export::qualification::current_affiliation::PinnedCurrentAffiliations;
use crate::export::qualification::participation::{
    ParticipationReplay, ParticipationReplayV3, PinnedParticipation,
};
use std::collections::BTreeMap;

#[test]
fn native_genuine_documentary_v3_archive_keeps_missing_facts_exact_bytes_and_outcomes() -> TestResult {
    let fixture = Fixture::open()?;
    let objects = tempfile::tempdir()?;
    let dataset = fixture.capture(objects.path())?;
    inspect(&dataset, &fixture.expected)?;
    let proof = serde_json::to_value(dataset.selected_proofs.as_ref().ok_or("selected proofs")?)?;
    let descriptor = proof.get("participation_digest").and_then(Value::as_str)
        .ok_or("participation descriptor")?;
    let current = ParticipationReplay::load(objects.path(), descriptor)?;
    check!(matches!(&current, ParticipationReplay::V4(_)));
    let mut wire = serde_json::to_value(&current)?;
    *wire.get_mut("version").ok_or("version")? = json!(3);
    let historical: ParticipationReplayV3 = serde_json::from_value(wire)?;
    let historical = ParticipationReplay::V3(historical);
    let expected_wire = serde_json::to_vec(&historical)?;
    let anchored = historical.install(objects.path())?;
    let expected_files = inventory(objects.path())?;
    let affiliations = PinnedCurrentAffiliations::read(&dataset, &fixture.store.snapshot())?;
    let expected = fixture.expected.clone();
    let profile = fixture.profile.clone();
    fixture.require_original_unchanged()?;
    fixture.close()?;
    for _ in 0..2 {
        let loaded = ParticipationReplay::load(objects.path(), &anchored)?;
        check!(matches!(&loaded, ParticipationReplay::V3(_)));
        check!(eq; serde_json::to_vec(&loaded)?, expected_wire);
        let token = PinnedParticipation::replay(&dataset, &profile, objects.path(), &anchored,
            &historical.fingerprints(), affiliations.roster_context())?;
        let (_, _, _, structured, documentary, unsupported, replay) = token.into_parts();
        check!(structured.is_empty());
        check!(unsupported.is_empty());
        check!(eq; documentary.len(), 3);
        for source in &expected {
            let row = documentary.get(&source.performance).ok_or("historical genuine row")?;
            check!(eq; row.domain_outcome(), CandidateOutcome::Unresolved);
            check!(eq; row.assessment().outcome(), ParticipationOutcome::Unresolved);
            check!(eq; row.event_id(), &source.event);
            check!(eq; row.athlete_id(), &source.athlete);
            check!(eq; row.assessment().reasons().iter().collect::<Vec<_>>(), vec![
                ParticipationReason::CohortUnresolved, ParticipationReason::ParticipationContextUnresolved,
                ParticipationReason::HighSchoolStatusMissing, ParticipationReason::SummerBoundaryUnresolved]);
            check!(eq; row.assessment().evidence_refs(), source.references);
            check!(eq; row.assessment().diagnostics(), &[ParticipationDiagnostic::DocumentaryAssessmentUnresolved,
                ParticipationDiagnostic::SchoolBindingUnresolved]);
        }
        check!(eq; serde_json::to_vec(&replay)?, expected_wire);
        check!(eq; inventory(objects.path())?, expected_files);
    }
    Ok(())
}

fn inventory(directory: &Path) -> TestResult<BTreeMap<PathBuf, Vec<u8>>> {
    std::fs::read_dir(directory)?.map(|entry| {
        let path = entry?.path();
        let bytes = std::fs::read(&path)?;
        Ok((path, bytes))
    }).collect()
}
