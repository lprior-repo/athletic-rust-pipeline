mod closure_grants;
mod durable_job;
mod parent_conflicts;

use super::documentary_material::Kind;
use super::documentary_review::{self, AcceptedSource};
use super::readback::{self, Expected};
use super::source::{self, InstalledSource, TestResult};
use crate::export::ExportDataset;
use crate::report::Scope;
use crate::workbook::publication;
use census_domain::evidence_subject_attachment::AttachmentPredecessorV1;
use census_domain::model::{GradYear, QueryProfileV1, SchoolId};
use census_domain::public_identity_history::PublicAthleteId;
use std::path::Path;

fn reviewed_inputs(installed: &InstalledSource) -> TestResult<(SchoolId, AcceptedSource)> {
    let current = super::current_school_source::install(installed)?;
    super::current_school_identity::accept(installed, &current)?;
    super::current_school_review::accept(installed, &current)?;
    let points = super::point_source::install(installed)?;
    let window = super::window_source::install(installed)?;
    let point50 = points.get(1).ok_or("controlled May15 source missing")?;
    super::school_authority::establish(installed, point50)?;
    let mut accepted50 = None;
    for (index, point) in points.iter().enumerate() {
        let accepted = documentary_review::accept(
            installed,
            point,
            Kind::Point,
            AttachmentPredecessorV1::First,
            &format!("controlled-status-{index}"),
        )?;
        if index == 1 {
            accepted50 = Some(accepted);
        }
    }
    documentary_review::accept(
        installed,
        &window,
        Kind::Window,
        AttachmentPredecessorV1::First,
        "controlled-full-window",
    )?;
    Ok((
        current.school,
        accepted50.ok_or("controlled May15 review missing")?,
    ))
}

fn assert_output(
    dataset: &ExportDataset,
    path: &Path,
    public: &PublicAthleteId,
    current: &SchoolId,
    historical: &SchoolId,
    second: bool,
) -> TestResult {
    let (seconds, day) = if second {
        ("51.00", "2026-05-16")
    } else {
        ("50.00", "2026-05-15")
    };
    readback::assert_generation(
        dataset,
        path,
        Expected {
            seconds,
            day,
            public,
            current_school: current,
            historical_school: historical,
        },
    )
}

fn publish(
    store: &census_store::Store,
    profile: &QueryProfileV1,
    objects: &Path,
    root: &Path,
) -> TestResult<(ExportDataset, std::path::PathBuf)> {
    let dataset = super::selection::capture(store, profile, objects)?;
    let path = super::publish::publish(&dataset, store, profile, root)?;
    Ok((dataset, path))
}

fn retain_outputs(g0: tempfile::TempDir, g1: tempfile::TempDir) -> TestResult {
    match std::env::var("CENSUS_CONTROLLED_THREE_MARKS_RETAIN") {
        Err(std::env::VarError::NotPresent) => Ok(()),
        Err(error) => Err(error.into()),
        Ok(value) if value == "1" => {
            println!("CONTROLLED_ONLY_G0={}", g0.keep().display());
            println!("CONTROLLED_ONLY_G1={}", g1.keep().display());
            Ok(())
        }
        Ok(_) => Err("CENSUS_CONTROLLED_THREE_MARKS_RETAIN must be 1 when set".into()),
    }
}

fn require_replay_selection(
    installed: &InstalledSource,
    profile: &QueryProfileV1,
    workbook: &Path,
    current: &SchoolId,
) -> TestResult {
    let objects = tempfile::tempdir()?;
    let dataset = super::selection::capture(&installed.store, profile, objects.path())?;
    assert_output(
        &dataset,
        workbook,
        &installed.public,
        current,
        &installed.school,
        true,
    )
}

#[test]
fn controlled_three_marks_selects_eligible_best_then_withdraws_to_next_and_reopens() -> TestResult {
    let installed = source::install()?;
    let (current, accepted50) = reviewed_inputs(&installed)?;
    let profile = super::selection::profile()?;
    installed.store.register_profile(&profile)?;
    let public = installed.public.clone();
    let historical = installed.school.clone();
    let objects0 = tempfile::tempdir()?;
    let objects1 = tempfile::tempdir()?;
    let output0 = tempfile::tempdir()?;
    let output1 = tempfile::tempdir()?;
    let (g0, workbook0) = publish(
        &installed.store,
        &profile,
        objects0.path(),
        &output0.path().join("publication"),
    )?;
    assert_output(&g0, &workbook0, &public, &current, &historical, false)?;
    super::proof_refusal::assert_required_proof_refuses(&g0)?;
    let lineage0 = g0.lineage.clone();
    let original_digest = publication::verify_published(&workbook0)?.generation_digest;
    super::documentary_lifecycle::revoke(&installed, &accepted50)?;
    let (g1, workbook1) = publish(
        &installed.store,
        &profile,
        objects1.path(),
        &output1.path().join("publication"),
    )?;
    let withdrawn_digest = publication::verify_published(&workbook1)?.generation_digest;
    assert_output(&g1, &workbook1, &public, &current, &historical, true)?;
    let lineage1 = g1.lineage.clone();
    check!(lineage1.snapshot_sequence > lineage0.snapshot_sequence);
    check!(publication::verify_published(&workbook1)?.generation_digest != original_digest);
    super::documentary_lifecycle::replay_without_reactivation(&installed, &accepted50)?;
    require_replay_selection(&installed, &profile, &workbook1, &current)?;
    check!(eq; publication::verify_published(&workbook0)?.generation_digest, original_digest);
    check!(eq; publication::verify_published(&workbook1)?.generation_digest, withdrawn_digest);
    source::require_originals(&installed.store, &installed.source_records)?;
    drop(g0);
    drop(g1);
    let InstalledSource {
        directory, store, ..
    } = installed;
    drop(store);
    directory.close()?;
    objects0.close()?;
    objects1.close()?;
    let old =
        publication::open_publication(&workbook0, &lineage0, Scope::AllSources, GradYear::CO2027)?;
    let new =
        publication::open_publication(&workbook1, &lineage1, Scope::AllSources, GradYear::CO2027)?;
    assert_output(
        &old.dataset,
        &workbook0,
        &public,
        &current,
        &historical,
        false,
    )?;
    assert_output(
        &new.dataset,
        &workbook1,
        &public,
        &current,
        &historical,
        true,
    )?;
    check!(eq; publication::verify_published(&workbook0)?.generation_digest, original_digest);
    retain_outputs(output0, output1)
}
