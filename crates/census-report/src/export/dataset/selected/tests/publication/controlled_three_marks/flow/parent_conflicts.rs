use super::super::documentary_material::Kind;
use super::super::documentary_review::{self, AcceptedSource};
use super::super::readback::Expected;
use super::super::source::{self, InstalledSource, TestResult};
use crate::export::ExportDataset;
use census_domain::model::{
    CanonicalTeam, Evidence, Gender, ProfileSelectionV1, SchoolYear, SourceRef, Sport,
};
use census_store::{Store, Table};
use sha2::{Digest, Sha256};
use std::path::Path;

fn capture(
    store: &Store,
    profile: &census_domain::model::QueryProfileV1,
    objects: &Path,
) -> TestResult<ExportDataset> {
    let selection = ProfileSelectionV1::new(store.store_identity()?, profile.id()?);
    let snapshot = store.snapshot();
    let dataset = ExportDataset::from_selected_snapshot(store, &snapshot, &selection, objects)?;
    dataset.ensure_current_selected_write()?;
    Ok(dataset)
}

fn performance_with_mark(
    dataset: &ExportDataset,
    mark: &str,
) -> TestResult<&census_domain::model::CanonicalPerformance> {
    dataset
        .performances
        .iter()
        .find(|p| p.mark == mark)
        .ok_or_else(|| format!("performance with mark {mark} not found"))
}

fn team_entity_id(team: &CanonicalTeam) -> &str {
    team.id.as_str()
}

#[test]
fn baseline_selected_capture_produces_expected_best() -> TestResult {
    let installed = source::install()?;
    let (_, accepted50) = super::super::flow::reviewed_inputs(&installed)?;
    let profile = super::super::selection::profile()?;
    installed.store.register_profile(&profile)?;

    let objects = tempfile::tempdir()?;
    let dataset = capture(&installed.store, &profile, objects.path())?;

    let performance = performance_with_mark(&dataset, "50.00")?;
    check!(
        performance.meet == "502100-controlled-meet-2026",
        "expected controlled May15 meet"
    );
    check!(performance.date == "2026-05-15", "expected controlled May15 date");

    let team = dataset
        .teams
        .get(&performance.team)
        .ok_or("performance team absent")?;
    check!(team.id != "controlled-divergent-program", "baseline has no injected team");

    let frozen = tempfile::tempdir()?;
    let path = frozen.path().join("controlled-baseline-frozen.json");
    dataset.save_frozen(&path)?;
    let original = std::fs::read(&path)?;
    let sha = format!("{:x}", Sha256::digest(&original));
    let reopened = ExportDataset::reopen_frozen_with_digest(&path, &sha)?;
    let reopened50 = performance_with_mark(&reopened, "50.00")?;
    check!(
        reopened50.athlete == performance.athlete,
        "frozen reopen preserves athlete"
    );
    check!(reopened50.mark == "50.00", "frozen reopen preserves mark");
    drop(accepted50);
    Ok(())
}

fn inject_parent_entity_id_conflict(store: &Store, team_id: &str) -> TestResult {
    let school_id = census_domain::model::Id::mint("school", &["controlled-divergent-school"]);
    let conflicting = CanonicalTeam {
        id: team_id.parse().map_err(|e| format!("controlled team id parse: {e}"))?,
        school: school_id,
        sport: Sport::OutdoorTrack,
        gender: Gender::Boys,
        school_year: SchoolYear::new(2026).map_err(|e| format!("controlled school year: {e}"))?,
        level: None,
        source_identities: vec![],
        evidence: vec![Evidence::parsed(
            SourceRef::new("controlled-injection", None),
            "2026-10-17T00:00:00Z",
        )],
        retained_conflicts: Vec::new(),
    };
    store.append(Table::Teams, &conflicting)?;
    Ok(())
}

#[test]
fn selected_capture_refuses_on_entity_id_parent_entity_id_collision() -> TestResult {
    let installed = source::install()?;
    let (_, accepted50) = super::super::flow::reviewed_inputs(&installed)?;
    let profile = super::super::selection::profile()?;
    installed.store.register_profile(&profile)?;

    let objects = tempfile::tempdir()?;
    let dataset = capture(&installed.store, &profile, objects.path())?;
    let performance = performance_with_mark(&dataset, "50.00")?;
    let team_id = performance.team.to_string();
    drop(dataset);

    inject_parent_entity_id_conflict(&installed.store, &team_id)?;

    let conflicting_objects = tempfile::tempdir()?;
    let result = capture(&installed.store, &profile, conflicting_objects.path());
    check!(result.is_err(), "expected capture refusal with parent entity id collision");
    let error = format!("{}", result.unwrap_err());
    check!(
        error.contains("parent") && error.contains("conflict"),
        "expected retained parent collision in error: {error}"
    );
    drop(accepted50);
    Ok(())
}

#[test]
fn frozen_clean_bound_export_survives_later_parent_entity_id_collision() -> TestResult {
    let installed = source::install()?;
    let (_, accepted50) = super::super::flow::reviewed_inputs(&installed)?;
    let profile = super::super::selection::profile()?;
    installed.store.register_profile(&profile)?;

    let objects = tempfile::tempdir()?;
    let dataset = capture(&installed.store, &profile, objects.path())?;
    let performance = performance_with_mark(&dataset, "50.00")?;
    let team_id = performance.team.to_string();

    let frozen = tempfile::tempdir()?;
    let path = frozen.path().join("controlled-clean-frozen.json");
    dataset.save_frozen(&path)?;
    let original = std::fs::read(&path)?;
    let sha = format!("{:x}", Sha256::digest(&original));
    let reopened = ExportDataset::reopen_frozen_with_digest(&path, &sha)?;
    let reopened50 = performance_with_mark(&reopened, "50.00")?;
    check!(
        reopened50.athlete == performance.athlete,
        "frozen clean export remains stable"
    );
    check!(reopened50.mark == "50.00", "frozen clean export preserves best");
    drop(reopened);

    inject_parent_entity_id_conflict(&installed.store, &team_id)?;

    let reopened_after_collision =
        ExportDataset::reopen_frozen_with_digest(&path, &sha)?;
    let reopened_after_50 = performance_with_mark(&reopened_after_collision, "50.00")?;
    check!(
        reopened_after_50.athlete == performance.athlete,
        "frozen export unaffected by later entity id collision"
    );
    check!(
        reopened_after_50.mark == "50.00",
        "frozen export preserves mark after later collision"
    );
    check!(
        std::fs::read(&path)? == original,
        "frozen bytes unchanged by later collision"
    );
    drop(reopened_after_collision);

    let later_objects = tempfile::tempdir()?;
    let later = capture(&installed.store, &profile, later_objects.path());
    check!(
        later.is_err(),
        "new capture after entity id collision still refuses"
    );
    drop(accepted50);
    Ok(())
}
