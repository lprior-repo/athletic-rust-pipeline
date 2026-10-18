use super::super::source::{self, TestResult};
use crate::export::ExportDataset;
use census_domain::model::{
    CanonicalTeam, Evidence, ExactSeconds, Gender, Mark, NaturalKey, ProfileSelectionV1,
    SourceNamespace, SourceRef,
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

fn performance_with_mark<'a>(
    dataset: &'a ExportDataset,
    mark: &str,
) -> TestResult<&'a census_domain::model::CanonicalPerformance> {
    let expected = Mark::TimeSeconds(ExactSeconds::parse(mark)?);
    dataset
        .performances
        .iter()
        .find(|p| p.mark == expected)
        .ok_or_else(|| format!("performance with mark {mark} not found").into())
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
    let mut matching_meets = dataset.meets.iter().filter(|meet| {
        meet.source_identities.iter().any(|identity| {
            identity.namespace == SourceNamespace::MilesplitMeet && identity.id == "502100"
        })
    });
    let expected_meet = matching_meets
        .next()
        .ok_or("controlled May15 meet absent")?;
    check!(
        matching_meets.next().is_none(),
        "controlled May15 meet ambiguous"
    );
    check!(
        performance.meet == expected_meet.id,
        "expected controlled May15 meet"
    );
    check!(
        performance.date == "2026-05-15",
        "expected controlled May15 date"
    );

    let team = dataset
        .teams
        .get(&performance.team)
        .ok_or("performance team absent")?;
    check!(
        team.id.as_str() != "controlled-divergent-program",
        "baseline has no injected team"
    );

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
    check!(
        reopened50.mark == Mark::TimeSeconds(ExactSeconds::parse("50.00")?),
        "frozen reopen preserves mark"
    );
    drop(accepted50);
    Ok(())
}

fn inject_parent_entity_id_conflict(store: &Store, baseline: &CanonicalTeam) -> TestResult {
    let mut conflicting = baseline.clone();
    conflicting.gender = match baseline.gender {
        Gender::Boys => Gender::Girls,
        Gender::Girls => Gender::Boys,
        Gender::Mixed => Gender::Boys,
        Gender::Unknown => Gender::Boys,
    };
    check!(!baseline.same_natural_key(&conflicting));
    check!(eq; baseline.id, conflicting.id);
    check!(eq; baseline.school, conflicting.school);
    conflicting.evidence.push(Evidence::parsed(
        SourceRef::new("controlled-injection", None),
        "2026-10-17T00:00:00Z",
    ));
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
    let baseline_team = dataset
        .teams
        .get(&performance.team)
        .ok_or("baseline parent team absent")?
        .clone();
    drop(dataset);

    inject_parent_entity_id_conflict(&installed.store, &baseline_team)?;

    let conflicting_objects = tempfile::tempdir()?;
    let result = capture(&installed.store, &profile, conflicting_objects.path());
    check!(
        result.is_err(),
        "expected capture refusal with parent entity id collision"
    );
    let error = match result {
        Err(error) => error.to_string(),
        Ok(_) => return Err("expected capture refusal with parent entity id collision".into()),
    };
    check!(
        error.contains("school-projected dataset parents retain a conflict"),
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
    let baseline_team = dataset
        .teams
        .get(&performance.team)
        .ok_or("baseline parent team absent")?
        .clone();

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
    check!(
        reopened50.mark == Mark::TimeSeconds(ExactSeconds::parse("50.00")?),
        "frozen clean export preserves best"
    );
    drop(reopened);

    inject_parent_entity_id_conflict(&installed.store, &baseline_team)?;

    let reopened_after_collision = ExportDataset::reopen_frozen_with_digest(&path, &sha)?;
    let reopened_after_50 = performance_with_mark(&reopened_after_collision, "50.00")?;
    check!(
        reopened_after_50.athlete == performance.athlete,
        "frozen export unaffected by later entity id collision"
    );
    check!(
        reopened_after_50.mark == Mark::TimeSeconds(ExactSeconds::parse("50.00")?),
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
