use std::error::Error;

use super::{key, ledger};
use crate::restate_services::wire::{
    JurisdictionRequest, StageOutcome, TeamsAttemptProgress, TeamsSourceOutcome, TeamsSourceRequest,
};
use crate::restate_services::JobError;
use census_domain::model::Revision;
use census_domain::{model::SchoolYear, UsJurisdiction};
use census_store::Store;

type TestResult = Result<(), Box<dyn Error>>;

mod cooldown;
mod durable_attempts;
mod gobound_policy;
pub(crate) mod national_resume;
mod native_deployment_handoff;
mod native_origin_continuation;
mod ordinary_pass_continuation;
mod origin_continuation_replay;
mod public_resources_fixture;
mod reconciliation_multi_source;
mod recorded_snapshot;
mod registration;
mod registration_cancellation;
mod replay;
mod retained_original_prefix;
mod settlement;

pub(in crate::restate_services::jurisdiction) fn request(
) -> Result<TeamsSourceRequest, Box<dyn Error>> {
    Ok(TeamsSourceRequest {
        jurisdiction: JurisdictionRequest {
            jurisdiction: UsJurisdiction::Ohio,
            season: SchoolYear::new(2026).ok_or("invalid school year")?,
            revision: Revision(1),
            history: crate::restate_services::wire::HistoryWindow::cohort("2026-10-02")?,
            refresh: false,
            limit_per_state: None,
            concurrency: 1,
            observed_on: None,
            authorized_hosts: Vec::new(),
            source_parallelism: 1,
        },
        source: "arbiter_orgs".to_string(),
        observed_on: "2026-10-02".to_string(),
    })
}

fn reserve(store: &Store, request: &TeamsSourceRequest) -> Result<(u8, String), Box<dyn Error>> {
    let operation = key(request);
    let registered = ledger::register(store, &operation, request)?;
    let baseline = ledger::baseline(store, &operation)?;
    match ledger::begin_at(store, &operation, request, &registered, baseline)? {
        ledger::AdmissionReplay::Reserved {
            attempt,
            observed_on,
        } => Ok((attempt, observed_on)),
        ledger::AdmissionReplay::Settled(_) => {
            Err("source unexpectedly settled before admission".into())
        }
        ledger::AdmissionReplay::ReplayRetry { message } => {
            Err(format!("source unexpectedly refused acquisition: {message}").into())
        }
    }
}

#[test]
fn transient_admissions_survive_reopen_and_exhaust_without_attempt_four() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    for expected in 1..=3 {
        let store = Store::open(root.path())?;
        let (attempt, _) = reserve(&store, &request)?;
        check!(eq; attempt, expected);
        let finished = ledger::finish(
            &store,
            &key(&request),
            attempt,
            (Err(JobError::Transient {
                message: format!("real source failure {attempt}"),
            }))
            .into(),
            &[],
        )?;
        if expected < 3 {
            check!(matches!(
                finished,
                ledger::Completion::Retry(JobError::Transient { .. })
            ));
        } else {
            check!(matches!(
                finished,
                ledger::Completion::Settled(TeamsSourceOutcome::Exhausted { attempts: 3, .. })
            ));
        }
    }
    let store = Store::open(root.path())?;
    check!(matches!(
        ledger::begin_at(
            &store,
            &key(&request),
            &request,
            &ledger::register(&store, &key(&request), &request)?,
            ledger::baseline(&store, &key(&request))?,
        )?,
        ledger::AdmissionReplay::Settled(TeamsSourceOutcome::Exhausted { attempts: 3, .. })
    ));
    check!(!store.journal_contains(
        ledger::PHASE,
        &format!("{}/attempt/4/reserved", key(&request))
    )?);
    Ok(())
}

#[test]
fn acknowledged_completion_replays_original_progress_and_date_after_midnight() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut request = request()?;
    let store = Store::open(root.path())?;
    let (attempt, observed_on) = reserve(&store, &request)?;
    let outcome = StageOutcome {
        records: 37,
        at: observed_on,
        errors: Vec::new(),
        notes: vec!["retained independently acquired coaches".to_string()],
        disposition: census_crawl::CollectionDisposition::Complete,
        unfinished: Vec::new(),
    };
    let expected = serde_json::to_value(&outcome)?;
    ledger::finish(&store, &key(&request), attempt, (Ok(outcome)).into(), &[])?;
    store.flush()?;
    drop(store);
    request.observed_on = "2026-10-03".to_string();
    request.jurisdiction.source_parallelism = 8;
    let store = Store::open(root.path())?;
    let ledger::AdmissionReplay::Settled(TeamsSourceOutcome::Completed { outcome, .. }) =
        ledger::begin_at(
            &store,
            &key(&request),
            &request,
            &ledger::register(&store, &key(&request), &request)?,
            ledger::baseline(&store, &key(&request))?,
        )?
    else {
        return Err("completed source became physically runnable".into());
    };
    check!(eq; serde_json::to_value(outcome)?, expected);
    check!(!store.journal_contains(
        ledger::PHASE,
        &format!("{}/attempt/2/reserved", key(&request))
    )?);
    Ok(())
}

#[test]
fn reserved_but_unacknowledged_attempts_are_consumed_and_never_claim_exhaustion() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    for expected in 1..=3 {
        let store = Store::open(root.path())?;
        check!(eq; reserve(&store, &request)?.0, expected);
    }
    let store = Store::open(root.path())?;
    check!(matches!(
        ledger::begin_at(
            &store,
            &key(&request),
            &request,
            &ledger::register(&store, &key(&request), &request)?,
            ledger::baseline(&store, &key(&request))?,
        )?,
        ledger::AdmissionReplay::Settled(TeamsSourceOutcome::Interrupted {
            attempts: Some(3),
            ..
        })
    ));
    check!(!store.journal_contains(
        ledger::PHASE,
        &format!("{}/attempt/4/reserved", key(&request))
    )?);
    Ok(())
}

#[test]
fn permanent_failure_stops_first_attempt_and_retains_its_reason() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    let store = Store::open(root.path())?;
    let (attempt, _) = reserve(&store, &request)?;
    ledger::finish(
        &store,
        &key(&request),
        attempt,
        (Err(JobError::Terminal {
            message: "public source access refused".to_string(),
        }))
        .into(),
        &[],
    )?;
    drop(store);
    let store = Store::open(root.path())?;
    let ledger::AdmissionReplay::Settled(TeamsSourceOutcome::Terminal { message, .. }) =
        ledger::begin_at(
            &store,
            &key(&request),
            &request,
            &ledger::register(&store, &key(&request), &request)?,
            ledger::baseline(&store, &key(&request))?,
        )?
    else {
        return Err("permanent failure became physically runnable".into());
    };
    check!(eq; message, "public source access refused");
    check!(!store.journal_contains(
        ledger::PHASE,
        &format!("{}/attempt/2/reserved", key(&request))
    )?);
    Ok(())
}

#[test]
fn changed_source_identity_cannot_reuse_existing_attempt_authority() -> TestResult {
    let root = tempfile::tempdir()?;
    let mut request = request()?;
    let operation = key(&request);
    let store = Store::open(root.path())?;
    reserve(&store, &request)?;
    let registered = ledger::register(&store, &operation, &request)?;
    let baseline = ledger::baseline(&store, &operation)?;
    request.source = "coach_directories".to_string();
    check!(matches!(
        ledger::begin_at(&store, &operation, &request, &registered, baseline),
        Err(JobError::Terminal { .. })
    ));
    Ok(())
}

#[test]
fn mixed_unknown_and_transient_attempts_never_claim_three_acknowledged_failures() -> TestResult {
    let root = tempfile::tempdir()?;
    let request = request()?;
    let store = Store::open(root.path())?;
    reserve(&store, &request)?;
    for expected in 2..=3 {
        let (attempt, _) = reserve(&store, &request)?;
        check!(eq; attempt, expected);
        ledger::finish(
            &store,
            &key(&request),
            attempt,
            (Err(JobError::Transient {
                message: format!("acknowledged failure {attempt}"),
            }))
            .into(),
            &[],
        )?;
    }
    let ledger::AdmissionReplay::Settled(TeamsSourceOutcome::Interrupted {
        attempts,
        progress,
        ..
    }) = ledger::begin_at(
        &store,
        &key(&request),
        &request,
        &ledger::register(&store, &key(&request), &request)?,
        ledger::baseline(&store, &key(&request))?,
    )?
    else {
        return Err("unknown physical outcome was misrepresented as exhausted failures".into());
    };
    check!(eq; attempts, Some(3));
    check!(matches!(
        progress.first(),
        Some(TeamsAttemptProgress::Unknown { attempt: 1 })
    ));
    check!(matches!(
        progress.last(),
        Some(TeamsAttemptProgress::Transient { attempt: 3, .. })
    ));
    Ok(())
}

#[test]
fn deliberate_foreign_state_exclusions_complete_teams_acquisition() -> TestResult {
    let mut report = census_crawl::AdapterReport::new("coach_directories", "schools");
    report.rows = 2;
    report.reject(
        "directory jurisdiction rejection https://example.test/directory: school SXBQNQ; disposition=foreign_published_state; requested=AL; published=FL; observed_on=2026-10-02; capture_sha256=abc",
    );
    report.finish_frontier();
    check!(eq; report.disposition, census_crawl::CollectionDisposition::Partial);
    let completed = super::stage(Some(report), "2026-10-02".to_string());
    let ledger::Attempt::Completed(outcome) = completed else {
        return Err("a deliberate foreign-state exclusion must not retry".into());
    };
    check!(eq; outcome.disposition, census_crawl::CollectionDisposition::Complete);
    check!(
        outcome.errors.is_empty(),
        "foreign exclusions are not acquisition errors: {:?}",
        outcome.errors
    );
    check!(
        outcome
            .notes
            .iter()
            .any(|note| note.contains("disposition=foreign_published_state")),
        "the exclusion remains on the stage notes"
    );
    let mut blocked = census_crawl::AdapterReport::new("coach_directories", "schools");
    blocked.reject("directory row https://example.test: no short code to fetch a summary with");
    blocked.finish_frontier();
    let refused = super::stage(Some(blocked), "2026-10-02".to_string());
    check!(matches!(refused, ledger::Attempt::Incomplete { .. }));
    Ok(())
}

mod access_evidence;

pub(in crate::restate_services::jurisdiction) fn bound_national_request(
    source: &TeamsSourceRequest,
) -> Result<crate::restate_services::wire::NationalRequest, Box<dyn Error>> {
    let mut national = crate::restate_services::national::publication_tests::fixtures::request()?;
    let jurisdiction = &source.jurisdiction;
    national.season = jurisdiction.season;
    national.revision = jurisdiction.revision;
    national.history = jurisdiction.history;
    national.jurisdictions = vec![jurisdiction.jurisdiction];
    national.refresh = jurisdiction.refresh;
    national.limit_per_state = jurisdiction.limit_per_state;
    national.concurrency = jurisdiction.concurrency;
    national.observed_on = jurisdiction.observed_on.clone();
    national.authorized_hosts = jurisdiction.authorized_hosts.clone();
    national.source_parallelism = jurisdiction.source_parallelism;
    Ok(national)
}
