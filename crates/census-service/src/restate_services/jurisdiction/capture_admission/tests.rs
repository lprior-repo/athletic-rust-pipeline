use super::*;
use crate::restate_services::plan::{compute_plan_fingerprint, BrowserLaneState};
use crate::restate_services::wire::{RefusedSource, SourcePlan};
use census_domain::model::Revision;
use census_domain::model::{HistoryWindow, SchoolYear};
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

struct Fixture {
    _root: tempfile::TempDir,
    store: Store,
    execution: CaptureExecution,
    request: JurisdictionRequest,
    state: JurisdictionState,
}

fn fixture() -> TestResult<Fixture> {
    let root = tempfile::tempdir()?;
    Store::create_capture_qualification(root.path())?;
    let (store, writer) = Store::open_current_qualification_v3(root.path())?.into_parts();
    let request = JurisdictionRequest {
        jurisdiction: UsJurisdiction::Michigan,
        season: SchoolYear::new(2026).ok_or("season")?,
        revision: Revision(1),
        history: HistoryWindow::new(2023, 2026, "2026-10-14")?,
        refresh: false,
        limit_per_state: None,
        concurrency: 1,
        observed_on: None,
        authorized_hosts: Vec::new(),
        source_parallelism: 1,
    };
    let key =
        WorkflowIdentity::jurisdiction(request.jurisdiction, request.season, request.revision);
    let state = JurisdictionState {
        identity: key.as_str().to_string(),
        history_window: Some(request.history),
        plan: Some(SourcePlan {
            sweepable: vec!["milesplit".into()],
            refused: Vec::new(),
            satisfied: Vec::new(),
            fingerprint: compute_plan_fingerprint(
                request.jurisdiction,
                request.season,
                request.revision,
                BrowserLaneState::Absent,
            ),
        }),
        ..JurisdictionState::default()
    };
    Ok(Fixture {
        _root: root,
        store,
        execution: CaptureExecution::FreshQualification(Arc::new(writer)),
        request,
        state,
    })
}

fn bind(fixture: &Fixture) -> TestResult {
    fixture.store.bind_run(&RunManifest {
        store_identity: fixture.store.store_identity()?.as_str().to_string(),
        run: CensusRun::new(fixture.request.season, fixture.request.revision.get()).ok_or("run")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![fixture.request.jurisdiction],
    })?;
    Ok(())
}

fn selection(state: &JurisdictionState) -> TestResult<SourceSelection> {
    SourceSelection::parse(&state.plan.as_ref().ok_or("plan")?.sweepable)
        .map_err(crate::restate_services::tests::sdk_error)
}

fn admit(f: &Fixture) -> Result<ResultsCaptureExecution, CaptureAdmissionError> {
    let selected = SourceSelection::parse(
        &f.state
            .plan
            .as_ref()
            .ok_or(CaptureAdmissionError::RoleUnsupported)?
            .sweepable,
    )
    .map_err(|_| CaptureAdmissionError::RoleUnsupported)?;
    admit_results_owner(
        &f.store,
        &f.execution,
        &f.state.identity,
        &f.request,
        &f.state,
        (2026, selected),
    )
}

#[test]
fn capture_owner_requires_the_actual_retained_manifest_key_and_window() -> TestResult {
    let mut f = fixture()?;
    check!(matches!(
        admit(&f),
        Err(CaptureAdmissionError::RunManifestMissing)
    ));
    bind(&f)?;
    check!(matches!(
        admit(&f)?,
        ResultsCaptureExecution::FreshQualification { .. }
    ));
    let original = f.state.identity.clone();
    f.state.identity = "jurisdiction:WI:2026-27:1".into();
    check!(matches!(
        admit(&f),
        Err(CaptureAdmissionError::OwnerKeyMismatch)
    ));
    f.state.identity = original;
    f.state.history_window = Some(HistoryWindow::new(2022, 2026, "2026-10-14")?);
    check!(matches!(
        admit(&f),
        Err(CaptureAdmissionError::HistoryConflict)
    ));
    Ok(())
}

#[test]
fn capture_owner_uses_sweepable_selection_and_retains_exact_plan_fingerprint() -> TestResult {
    let mut f = fixture()?;
    bind(&f)?;
    let plan = f.state.plan.as_mut().ok_or("plan")?;
    plan.sweepable.clear();
    plan.refused.push(RefusedSource {
        slug: "milesplit".into(),
        reason: "excluded".into(),
        kind: crate::restate_services::plan::RefusalKind::EngineeringGap,
    });
    check!(matches!(
        admit(&f)?,
        ResultsCaptureExecution::UnadoptedSources
    ));
    let stale = SourceSelection::parse(&["milesplit".into()])
        .map_err(crate::restate_services::tests::sdk_error)?;
    check!(matches!(
        admit_results_owner(
            &f.store,
            &f.execution,
            &f.state.identity,
            &f.request,
            &f.state,
            (2026, stale)
        ),
        Err(CaptureAdmissionError::RoleUnsupported)
    ));
    let plan = f.state.plan.as_mut().ok_or("plan")?;
    plan.sweepable = vec!["milesplit".into()];
    let fingerprint = plan.fingerprint.clone();
    let manifest = f.store.run_manifest()?.ok_or("manifest")?;
    let scoped = scope(&f.state.identity, &f.request, &manifest, 2026, &fingerprint)?;
    check!(eq; scoped.source_admission().plan_fingerprint().as_str(), fingerprint);
    check!(eq; selection(&f.state)?.iter().count(), 1);
    Ok(())
}

#[test]
fn capture_manifest_rejects_foreign_lineage_cohort_scope_and_run() -> TestResult {
    let f = fixture()?;
    bind(&f)?;
    let manifest = f.store.run_manifest()?.ok_or("manifest")?;
    let mut foreign = manifest.clone();
    foreign.store_identity = "a".repeat(64);
    check!(matches!(
        require_bound_manifest(&f.execution, Some(&foreign), &f.request),
        Err(CaptureAdmissionError::RunMismatch)
    ));
    foreign = manifest.clone();
    foreign.jurisdictions.clear();
    check!(matches!(
        require_bound_manifest(&f.execution, Some(&foreign), &f.request),
        Err(CaptureAdmissionError::JurisdictionOutsideManifest)
    ));
    foreign = manifest.clone();
    foreign.run = CensusRun::new(f.request.season, 2).ok_or("run")?;
    check!(matches!(
        require_bound_manifest(&f.execution, Some(&foreign), &f.request),
        Err(CaptureAdmissionError::RunMismatch)
    ));
    foreign = manifest;
    foreign.cohort = census_domain::model::GradYear::new(2028).ok_or("cohort")?;
    check!(matches!(
        require_bound_manifest(&f.execution, Some(&foreign), &f.request),
        Err(CaptureAdmissionError::RunMismatch)
    ));
    check!(require_bound_manifest(&CaptureExecution::LegacyFresh, None, &f.request).is_ok());
    Ok(())
}

#[path = "attempts.rs"]
mod attempts;

#[test]
fn a_history_subset_without_milesplit_does_not_mint_or_require_its_role() -> TestResult {
    let f = fixture()?;
    bind(&f)?;
    let subset = SourceSelection::parse(&["ihsa".into()])
        .map_err(crate::restate_services::tests::sdk_error)?;
    let admitted = admit_results_owner(
        &f.store,
        &f.execution,
        &f.state.identity,
        &f.request,
        &f.state,
        (2026, subset),
    )?;
    check!(matches!(
        admitted,
        ResultsCaptureExecution::UnadoptedSources
    ));
    Ok(())
}
