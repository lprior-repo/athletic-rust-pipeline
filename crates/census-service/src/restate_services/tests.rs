use census_crawl::{net::FetchError, CrawlError};
use std::num::NonZeroUsize;
use std::path::PathBuf;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;

use crate::census::{owed_source_objects, MeetCensus, SourceObject, StateProgress};
use census_crawl::registry::{
    AccessClass, SourceAdmission, SourceCapabilities, SourceDescriptor, TransportKind,
};
use census_reconcile::identity::{Revision, WorkflowIdentity};
use census_report::report::ReportError;
use census_store::StoreError;

use super::*;
use crate::restate_services::ingest::payload_digest;
use crate::restate_services::plan::classify_access;
use crate::restate_services::results_arms::ResultsStageOutcome;
use census_report::report::Scope;
use census_store::Table;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[derive(Debug)]
struct FixtureSdkError(HandlerError);

impl std::fmt::Display for FixtureSdkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let error: &dyn std::error::Error = self.0.as_ref();
        std::fmt::Display::fmt(error, formatter)
    }
}

impl std::error::Error for FixtureSdkError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.0.as_ref())
    }
}

pub(super) fn sdk_error(error: impl Into<HandlerError>) -> Box<dyn std::error::Error> {
    Box::new(FixtureSdkError(error.into()))
}

fn panic_blocking_job_fault() -> ! {
    panic!("boom");
}

fn invalid_json() -> TestResult<serde_json::Error> {
    match serde_json::from_str::<serde_json::Value>("not json") {
        Err(error) => Ok(error),
        Ok(_) => Err("invalid JSON fixture parsed".into()),
    }
}

#[test]
fn table_names_resolve_and_reject_typos() -> TestResult {
    check!(eq; resolve_table("schools").map_err(sdk_error)?, Table::Schools);
    check!(eq; resolve_table("performances").map_err(sdk_error)?, Table::Performances);
    check!(resolve_table("school").is_err());
    check!(resolve_table("").is_err());
    Ok(())
}

#[test]
fn empty_table_list_means_every_table_in_order() -> TestResult {
    check!(eq; resolve_tables(&[]).map_err(sdk_error)?, Table::ALL.to_vec());
    let requested = vec!["meets".to_string(), "meets".to_string()];
    check!(eq; resolve_tables(&requested).map_err(sdk_error)?, vec![Table::Meets]);
    Ok(())
}

#[test]
fn an_omitted_scope_matches_the_cli_default_and_unknown_scopes_are_rejected() -> TestResult {
    check!(eq; resolve_scope(None).map_err(sdk_error)?, Scope::AllSources);
    check!(eq; resolve_scope(Some("core")).map_err(sdk_error)?, Scope::Core);
    check!(eq;
        resolve_scope(Some("all_sources")).map_err(sdk_error)?,
        Scope::AllSources
    );
    check!(resolve_scope(Some("all")).is_err());
    Ok(())
}

#[test]
fn cohort_label_names_the_reduction() {
    assert_eq!(cohort_label(Some(2027)), "co2027");
    assert_eq!(cohort_label(None), "all");
}

fn physical(store: &Store, table: Table) -> TestResult<(u64, u64)> {
    Ok((store.walk_table(table)?.rows, store.receipt_count()?))
}

#[test]
fn three_attempts_at_one_operation_append_it_once_and_leave_one_receipt() -> TestResult {
    let dir = tempfile::tempdir()?;
    let operation = "wiaa_results_wi:inv-1:2026-W39:performances:0:0";
    let rows = vec![
        serde_json::json!({"id": "perf:wi:1", "mark": "10.94"}),
        serde_json::json!({"id": "perf:wi:2", "mark": "11.02"}),
    ];
    let digest = payload_digest(Table::Performances, &rows).map_err(sdk_error)?;

    let store = Store::open(dir.path())?;
    let first = apply_observations(&store, Table::Performances, &rows, operation, &digest)?;
    check!(eq;
        first.appended(),
        rows.len() as u64,
        "the first attempt appends the page: {first:?}"
    );
    let after_first = physical(&store, Table::Performances)?;
    check!(eq; after_first, (2, 1), "two rows, one receipt");
    drop(store);

    let store = Store::open(dir.path())?;
    for attempt in 2..=3 {
        let again =
            apply_observations(&store, Table::Performances, &rows, operation, &digest)?.appended();
        check!(eq; again, 0, "attempt {attempt} appended nothing");
        check!(eq;
            physical(&store, Table::Performances)?,
            after_first,
            "attempt {attempt} left the store exactly as attempt 1 did"
        );
    }

    let receipt = store
        .receipt(operation)?
        .ok_or("missing operation receipt")?;
    check!(eq;
        receipt.appended, 2,
        "the receipt describes the one application"
    );
    check!(eq; receipt.digest, digest);
    Ok(())
}

#[test]
fn a_second_operation_with_different_rows_is_not_mistaken_for_a_replay() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let rows = vec![serde_json::json!({"id": "perf:wi:1", "mark": "10.94"})];
    let digest = payload_digest(Table::Performances, &rows).map_err(sdk_error)?;

    let first = apply_observations(&store, Table::Performances, &rows, "op-1", &digest)?;
    check!(eq; first.appended(), 1);
    let second = apply_observations(&store, Table::Performances, &rows, "op-2", &digest)?;
    check!(eq;
        second.appended(),
        1,
        "a different operation appends its own page, overlapping rows and all"
    );
    check!(eq; physical(&store, Table::Performances)?, (2, 2));
    Ok(())
}

#[test]
fn rows_without_an_id_are_rejected_by_the_store_and_nothing_is_written() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let rows = vec![serde_json::json!({"name": "no id here"})];
    check!(apply_observations(&store, Table::Schools, &rows, "op-1", "digest-1").is_err());
    check!(eq; store.stats()?.observations, 0);
    check!(eq;
        store.receipt_count()?,
        0,
        "a rejected page leaves no receipt behind"
    );

    let good = vec![serde_json::json!({"id": "school:wi:test", "name": "Test"})];
    check!(eq;
        apply_observations(&store, Table::Schools, &good, "op-2", "digest-2")?.appended(),
        1
    );
    check!(eq; store.stats()?.observations, 1);
    check!(eq;
        apply_observations(&store, Table::Schools, &good, "op-2", "digest-2")?.appended(),
        0,
        "the same operation twice appends its page once"
    );
    check!(eq; store.stats()?.observations, 1);
    Ok(())
}

#[test]
fn a_derived_page_is_written_by_id_and_a_replay_writes_nothing() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let operation = "athleticnet_wi:inv-2:2026-W39:review_cases:0:0";
    let rows = vec![
        serde_json::json!({
            "id": "case:1",
            "family": "Athlete identity",
            "subject_id": "subject:1",
            "subject": "A",
            "detail": "shared name",
            "state": "pending"
        }),
        serde_json::json!({
            "id": "case:2",
            "family": "Athlete identity",
            "subject_id": "subject:2",
            "subject": "B",
            "detail": "shared name",
            "state": "pending"
        }),
    ];
    let digest = payload_digest(Table::ReviewCases, &rows).map_err(sdk_error)?;
    let first = apply_observations(&store, Table::ReviewCases, &rows, operation, &digest)?;
    check!(eq;
        first.appended(),
        0,
        "a derived page appends no observation: {first:?}"
    );
    check!(eq;
        store.scan::<census_domain::model::ReviewCase>(Table::ReviewCases)?.len(),
        2
    );
    check!(eq;
        store.walk_table(Table::ReviewCases)?.foreign_sequences,
        0,
        "the page lands under the table's own keys"
    );
    check!(eq; store.stats()?.observations, 0);
    let replay = apply_observations(&store, Table::ReviewCases, &rows, operation, &digest)?;
    check!(replay.repeated(), "the same operation repeats: {replay:?}");
    check!(eq;
        store.scan::<census_domain::model::ReviewCase>(Table::ReviewCases)?.len(),
        2,
        "a replay writes no duplicate row"
    );
    let overlapping = apply_observations(&store, Table::ReviewCases, &rows, "op-3", &digest)?;
    check!(eq;
        overlapping.appended(),
        0,
        "a second operation replaces the same ids in place: {overlapping:?}"
    );
    check!(eq;
        store.scan::<census_domain::model::ReviewCase>(Table::ReviewCases)?.len(),
        2
    );
    check!(store.integrity()?.ok);
    Ok(())
}

#[test]
fn oversized_batches_are_refused_without_touching_the_store() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let rows = vec![serde_json::json!({"id": "x"}); MAX_ROWS_PER_REQUEST + 1];
    let refused = match apply_observations(&store, Table::Schools, &rows, "op-1", "digest-1")
        .map_err(JobError::from)
    {
        Err(error) => error,
        Ok(_) => return Err("oversized batch accepted".into()),
    };
    check!(matches!(refused, JobError::Terminal { .. }));
    check!(eq; store.stats()?.observations, 0);
    check!(eq;
        store.receipt_count()?,
        0,
        "the refusal happens before the commit, so no receipt is written"
    );
    Ok(())
}

#[test]
fn job_failures_classify_for_retry_but_a_violated_invariant_and_a_panic_never_retry() -> TestResult
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let region = Arc::new(Spawner::new());

    let refused = runtime.block_on(blocking(Arc::clone(&region), || {
        Err::<u8, StoreError>(StoreError::Invariant {
            detail: "50001 rows exceeds the per-request ceiling of 50000".to_string(),
        })
    }));
    check!(matches!(refused, Err(JobError::Terminal { .. })));

    let transient = runtime.block_on(blocking(Arc::clone(&region), || {
        Err::<u8, StoreError>(StoreError::Io {
            path: PathBuf::from("/dev/null/nowhere"),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        })
    }));
    check!(matches!(transient, Err(JobError::Transient { .. })));

    let report = runtime.block_on(blocking(Arc::clone(&region), || {
        Err::<u8, ReportError>(ReportError::Store(StoreError::Invariant {
            detail: "table schools would exceed 20000000 rows in one scan".to_string(),
        }))
    }));
    check!(matches!(report, Err(JobError::Terminal { .. })));

    let report_invariant = runtime.block_on(blocking(Arc::clone(&region), || {
        Err::<u8, ReportError>(ReportError::Invariant {
            detail: "the core scope reports more than the all-sources scope".to_string(),
        })
    }));
    check!(matches!(report_invariant, Err(JobError::Terminal { .. })));

    let report_io = runtime.block_on(blocking(Arc::clone(&region), || {
        Err::<u8, ReportError>(ReportError::Io {
            path: PathBuf::from("/dev/null/nowhere"),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        })
    }));
    check!(matches!(report_io, Err(JobError::Transient { .. })));

    let panicked = runtime.block_on(blocking(Arc::clone(&region), || -> Result<u8, JobError> {
        panic_blocking_job_fault()
    }));
    check!(matches!(panicked, Err(JobError::Terminal { .. })));
    Ok(())
}

fn national_request(jurisdictions: Vec<UsJurisdiction>) -> TestResult<NationalRequest> {
    Ok(NationalRequest {
        season: SchoolYear::new(2026).ok_or("invalid fixture season")?,
        revision: Revision(1),
        jurisdictions,
        refresh: false,
        limit_per_state: None,
        concurrency: 4,
        observed_on: None,
        authorized_hosts: Vec::new(),
        source_parallelism: census_crawl::net::DEFAULT_FAMILY_PARALLELISM,
        school_address: None,
    })
}

#[test]
fn an_empty_jurisdiction_list_covers_the_census_scope_in_declaration_order() -> TestResult {
    let targets = national::targets(&national_request(Vec::new())?).map_err(sdk_error)?;
    check!(eq; targets.len(), UsJurisdiction::CENSUS_SCOPE.len());
    let jurisdictions: Vec<UsJurisdiction> = targets.iter().map(|row| row.0).collect();
    check!(eq; jurisdictions, UsJurisdiction::CENSUS_SCOPE.to_vec());
    for (jurisdiction, key) in &targets {
        check!(eq;
            key.as_str(),
            WorkflowIdentity::jurisdiction(
                *jurisdiction,
                SchoolYear::new(2026).ok_or("invalid fixture season")?,
                Revision(1),
            )
            .as_str()
        );
    }
    Ok(())
}

#[test]
fn a_named_jurisdiction_set_keeps_the_callers_order() -> TestResult {
    let targets = national::targets(&national_request(vec![
        UsJurisdiction::Iowa,
        UsJurisdiction::Wisconsin,
    ])?)
    .map_err(sdk_error)?;
    check!(eq; targets.len(), 2);
    check!(eq; targets[0].0, UsJurisdiction::Iowa);
    check!(eq; targets[0].1, "jurisdiction:IA:2026-27:1");
    check!(eq; targets[1].0, UsJurisdiction::Wisconsin);
    check!(eq; targets[1].1, "jurisdiction:WI:2026-27:1");
    Ok(())
}

#[test]
fn a_state_named_twice_is_refused_instead_of_walked_twice() -> TestResult {
    let error = match national::targets(&national_request(vec![
        UsJurisdiction::Wisconsin,
        UsJurisdiction::Iowa,
        UsJurisdiction::Wisconsin,
    ])?) {
        Err(error) => error,
        Ok(_) => return Err("duplicate jurisdiction accepted".into()),
    };
    check!(
        format!("{error:?}").contains("WI"),
        "the refusal must name the repeated state: {error:?}"
    );
    Ok(())
}

#[test]
fn the_walk_options_carry_the_runs_shared_knobs() -> TestResult {
    let mut request = national_request(vec![UsJurisdiction::Iowa])?;
    request.limit_per_state = Some(17);
    request.concurrency = 3;
    request.refresh = true;
    let projected = request.for_jurisdiction(UsJurisdiction::Iowa);
    let options = super::options_for_request(&projected, "2026-09-21").map_err(sdk_error)?;
    check!(eq; options.jurisdictions, vec![UsJurisdiction::Iowa]);
    check!(eq; options.limit_per_state, Some(17));
    check!(eq; options.concurrency, 3);
    check!(eq; options.state_concurrency, 1);
    check!(options.refresh);
    check!(eq;
        options.school_year,
        SchoolYear::new(2026).ok_or("invalid fixture season")?
    );
    check!(eq; options.observed_on, "2026-09-21");
    Ok(())
}

#[test]
fn the_collection_date_is_the_days_today_unless_the_request_names_one() -> TestResult {
    let request = national_request(vec![UsJurisdiction::Iowa])?;
    let today = super::options_for_request(
        &request.for_jurisdiction(UsJurisdiction::Iowa),
        "2026-09-21",
    )
    .map_err(sdk_error)?;
    check!(eq; today.observed_on, "2026-09-21");

    let mut dated = request;
    dated.observed_on = Some("2026-09-01".to_string());
    let named =
        super::options_for_request(&dated.for_jurisdiction(UsJurisdiction::Iowa), "2026-09-21")
            .map_err(sdk_error)?;
    check!(eq; named.observed_on, "2026-09-01");
    Ok(())
}

#[test]
fn the_roster_ceiling_admits_its_boundary_and_refuses_one_past_it() -> TestResult {
    let mut request = national_request(vec![UsJurisdiction::Iowa])?;
    request.limit_per_state = Some(MAX_LIMIT_PER_STATE);
    let projected = request.for_jurisdiction(UsJurisdiction::Iowa);
    check!(eq;
        super::options_for_request(&projected, "2026-09-21").map_err(sdk_error)?.limit_per_state,
        Some(MAX_LIMIT_PER_STATE)
    );

    let mut over = national_request(vec![UsJurisdiction::Iowa])?;
    over.limit_per_state = Some(MAX_LIMIT_PER_STATE.saturating_add(1));
    let projected = over.for_jurisdiction(UsJurisdiction::Iowa);
    let error = match super::options_for_request(&projected, "2026-09-21") {
        Err(error) => error,
        Ok(_) => return Err("excessive roster ceiling accepted".into()),
    };
    check!(
        format!("{error:?}").contains("limit_per_state"),
        "the refusal must name the knob: {error:?}"
    );
    Ok(())
}

#[test]
fn a_walk_with_no_concurrency_is_refused() -> TestResult {
    let mut request = national_request(vec![UsJurisdiction::Iowa])?;
    request.concurrency = 0;
    let projected = request.for_jurisdiction(UsJurisdiction::Iowa);
    let error = match super::options_for_request(&projected, "2026-09-21") {
        Err(error) => error,
        Ok(_) => return Err("zero concurrency accepted".into()),
    };
    check!(
        format!("{error:?}").contains("concurrency"),
        "the refusal must name the knob: {error:?}"
    );
    Ok(())
}

fn answered_report() -> JurisdictionReport {
    JurisdictionReport {
        identity: "jurisdiction:WI:2026-27:1".to_string(),
        jurisdiction: UsJurisdiction::Wisconsin,
        plan: SourcePlan::of(
            &plan(UsJurisdiction::Wisconsin, BrowserLaneState::Absent),
            String::new(),
        ),
        stages_run: vec!["teams".to_string(), "rosters".to_string()],
        teams: 7,
        rosters: StateProgress {
            jurisdiction: UsJurisdiction::Wisconsin,
            teams: 7,
            rosters_total: 7,
            rosters_committed: 5,
            rosters_remaining: 2,
            rosters_skipped: 0,
            athletes: 11,
            class_of_2027: 3,
            class_of_2027_boys: 2,
            class_of_2027_girls: 1,
            errors: Vec::new(),
            blocked: false,
            blocked_skipped: 0,
        },
        consolidated: Vec::new(),
        meets: MeetCensus::default(),
        results: ResultsStageOutcome::default(),
        completed_at: "2026-09-22".to_string(),
    }
}

#[test]
fn a_state_that_did_not_answer_becomes_a_failure_row_and_the_run_keeps_its_summaries() -> TestResult
{
    let key = "jurisdiction:IA:2026-27:1";
    let failed = national::classify(
        UsJurisdiction::Iowa,
        key,
        Err(TerminalError::new("index host refused the walk")),
    );
    let national::Completion::Unanswered(failure) = failed else {
        return Err("failed call classified as a summary".into());
    };
    check!(eq; failure.jurisdiction, UsJurisdiction::Iowa);
    check!(eq;
        failure.identity, key,
        "the row must name the identity the run addressed"
    );
    check!(
        failure.error.contains("index host refused the walk"),
        "the row must say why the state is missing: {}",
        failure.error
    );

    let answered = national::classify(
        UsJurisdiction::Wisconsin,
        "jurisdiction:WI:2026-27:1",
        Ok(Json(answered_report())),
    );
    let national::Completion::Answered(summary) = answered else {
        return Err("returned report classified as a failure".into());
    };
    check!(eq; summary.jurisdiction, UsJurisdiction::Wisconsin);
    check!(eq; summary.identity, "jurisdiction:WI:2026-27:1");
    check!(eq; summary.rosters_total, 7);
    check!(eq; summary.rosters_committed, 5);
    check!(eq; summary.rosters_skipped, 0);
    check!(eq; summary.class_of_2027, 3);
    check!(eq;
        summary.rosters_remaining, 2,
        "seven teams with five rosters walked leave two remaining"
    );
    check!(eq; summary.athletes, 11);
    check!(!summary.blocked, "a completed walk is not a blocked one");
    Ok(())
}

static BROWSER_ONLY: SourceDescriptor = SourceDescriptor {
    slug: "browser-only",
    provider: "a surface that only renders in a session",
    transport: TransportKind::Browser,
    capabilities: SourceCapabilities {
        athlete_discovery: false,
        athlete_profile: false,
        meet_discovery: false,
        bulk_results: false,
        grade_evidence: false,
        graduation_evidence: false,
        school_evidence: false,
        coach_directory: false,
        public_professional_contact: false,
        pr_evidence: false,
    },
    admission: SourceAdmission {
        origin: "browser-only.test",
        target_requests_per_second: 1.0,
        maximum_in_flight: NonZeroUsize::MIN,
        robots_crawl_delay_respected: false,
    },
};

#[test]
fn a_refused_source_is_owed_evidence_and_is_never_dispatched() {
    assert_eq!(
        BROWSER_ONLY.access_class(),
        AccessClass::BrowserSession,
        "a browser transport on a non-artifact origin is a browser session"
    );

    let without_lane = [classify_access(
        BROWSER_ONLY.slug,
        BROWSER_ONLY.access_class(),
        Dispatch::Wired,
        BrowserLaneState::Absent,
    )];
    let with_lane = [classify_access(
        BROWSER_ONLY.slug,
        BROWSER_ONLY.access_class(),
        Dispatch::Wired,
        BrowserLaneState::Configured,
    )];

    let refusals = owed(&without_lane);
    assert_eq!(
        refusals.len(),
        1,
        "one applicable source this machine cannot run, one refusal"
    );
    assert_eq!(
        refusals[0].slug, BROWSER_ONLY.slug,
        "the refusal must name the source it refuses"
    );
    assert_eq!(
        refusals[0].access,
        AccessClass::BrowserSession,
        "and record how it would have been acquired"
    );
    assert!(
        refusals[0].reason.contains("browser lane"),
        "the reason must name what is missing, got {:?}",
        refusals[0].reason
    );
    assert!(
        sweepable(&without_lane).is_empty(),
        "a refused unit must not be handed to a dispatcher"
    );

    assert_eq!(
        sweepable(&with_lane).len(),
        1,
        "a configured lane makes the same source an ordinary unit"
    );
    assert!(
        owed(&with_lane).is_empty(),
        "and then no work is owed for it"
    );

    let recorded = SourceObject {
        endpoint: BROWSER_ONLY.slug.to_string(),
        observations: 0,
        windows: 0,
    };
    assert_eq!(
        owed_source_objects(&[recorded]),
        1,
        "a source object that accepted no observation is owed work"
    );
}

#[test]
fn fetch_error_retryable_variants_become_transient() {
    use census_crawl::net::FetchError;

    assert!(FetchError::Timeout {
        url: "https://example.com".to_string(),
        timeout_secs: 30,
    }
    .retryable());

    assert!(FetchError::RateLimited {
        url: "https://example.com".to_string(),
        retry_after_secs: None,
    }
    .retryable());

    assert!(FetchError::Http {
        status: 500,
        url: "https://example.com".to_string(),
    }
    .retryable());

    assert!(FetchError::Http {
        status: 429,
        url: "https://example.com".to_string(),
    }
    .retryable());

    assert!(FetchError::BrowserLane {
        url: "https://example.com".to_string(),
        detail: "profile busy".to_string(),
        retryable: true,
    }
    .retryable());

    let timeout = collect_error(CrawlError::Fetch(FetchError::Timeout {
        url: "https://example.com".to_string(),
        timeout_secs: 30,
    }));
    assert!(matches!(timeout, JobError::Transient { .. }));

    let rate_limited = collect_error(CrawlError::Fetch(FetchError::RateLimited {
        url: "https://example.com".to_string(),
        retry_after_secs: None,
    }));
    assert!(matches!(rate_limited, JobError::Transient { .. }));

    let http_500 = collect_error(CrawlError::Fetch(FetchError::Http {
        status: 500,
        url: "https://example.com".to_string(),
    }));
    assert!(matches!(http_500, JobError::Transient { .. }));

    let http_429 = collect_error(CrawlError::Fetch(FetchError::Http {
        status: 429,
        url: "https://example.com".to_string(),
    }));
    assert!(matches!(http_429, JobError::Transient { .. }));

    let browser_retryable = collect_error(CrawlError::Fetch(FetchError::BrowserLane {
        url: "https://example.com".to_string(),
        detail: "profile busy".to_string(),
        retryable: true,
    }));
    assert!(matches!(browser_retryable, JobError::Transient { .. }));
}

#[test]
fn fetch_error_nonretryable_variants_become_terminal() -> TestResult {
    use census_crawl::net::FetchError;

    let too_large = collect_error(CrawlError::Fetch(FetchError::TooLarge {
        url: "https://example.com".to_string(),
    }));
    check!(
        matches!(too_large, JobError::Terminal { .. }),
        "TooLarge must be terminal"
    );

    let browser_not_retryable = collect_error(CrawlError::Fetch(FetchError::BrowserLane {
        url: "https://example.com".to_string(),
        detail: "human_required".to_string(),
        retryable: false,
    }));
    check!(
        matches!(browser_not_retryable, JobError::Terminal { .. }),
        "BrowserLane {{ retryable: false }} must be terminal (the defect was it became Transient)"
    );

    let invalid_url = collect_error(CrawlError::Fetch(FetchError::InvalidUrl {
        url: "not a url".to_string(),
        source: match url::Url::parse("not a url") {
            Err(error) => error,
            Ok(_) => return Err("invalid URL fixture parsed".into()),
        },
    }));
    check!(matches!(invalid_url, JobError::Terminal { .. }));

    let http_404 = collect_error(CrawlError::Fetch(FetchError::Http {
        status: 404,
        url: "https://example.com".to_string(),
    }));
    check!(matches!(http_404, JobError::Terminal { .. }));

    let http_403 = collect_error(CrawlError::Fetch(FetchError::Http {
        status: 403,
        url: "https://example.com".to_string(),
    }));
    check!(matches!(http_403, JobError::Terminal { .. }));
    Ok(())
}

#[test]
fn non_fetch_crawl_errors_are_terminal() -> TestResult {
    let schema = collect_error(CrawlError::Schema {
        url: "https://example.com".to_string(),
        detail: "missing field".to_string(),
    });
    check!(matches!(schema, JobError::Terminal { .. }));

    let decode = collect_error(CrawlError::Decode {
        url: "https://example.com".to_string(),
        source: invalid_json()?,
    });
    check!(matches!(decode, JobError::Terminal { .. }));

    let domain = collect_error(CrawlError::Domain(census_domain::DomainError::OutOfRange {
        field: "grad_year",
    }));
    check!(matches!(domain, JobError::Terminal { .. }));

    let arithmetic = collect_error(CrawlError::Arithmetic {
        detail: "overflow".to_string(),
    });
    check!(matches!(arithmetic, JobError::Terminal { .. }));

    let io_err = collect_error(CrawlError::Io {
        path: std::path::PathBuf::from("/dev/null/nowhere"),
        source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
    });
    check!(matches!(io_err, JobError::Terminal { .. }));

    let encode = collect_error(CrawlError::Encode {
        table: "schools".to_string(),
        source: serde_json::Error::io(std::io::Error::from(std::io::ErrorKind::Other)),
    });
    check!(matches!(encode, JobError::Terminal { .. }));
    Ok(())
}

#[test]
fn deterministic_store_errors_classify_terminal() -> TestResult {
    let error = JobError::from(StoreError::CounterOverflow);
    check!(matches!(error, JobError::Terminal { .. }));

    let error = JobError::from(StoreError::Decode {
        key: "schools:1".to_string(),
        source: invalid_json()?,
    });
    check!(matches!(error, JobError::Terminal { .. }));

    let error = JobError::from(StoreError::Json {
        detail: "raw row".to_string(),
        source: invalid_json()?,
    });
    check!(matches!(error, JobError::Terminal { .. }));

    let error = JobError::from(StoreError::SnapshotRow {
        path: PathBuf::from("/tmp/out/schools.jsonl"),
        line: 42,
        source: invalid_json()?,
    });
    check!(matches!(error, JobError::Terminal { .. }));

    let error = JobError::from(StoreError::TooManyRows {
        table: "schools".to_string(),
        max: 20_000_000,
    });
    check!(matches!(error, JobError::Terminal { .. }));

    let error = JobError::from(StoreError::JournalTooLarge {
        what: "value",
        phase: "teams".to_string(),
        key: "wi:1".to_string(),
        bytes: 1_000_000,
        max: 500_000,
    });
    check!(matches!(error, JobError::Terminal { .. }));

    let error = JobError::from(StoreError::Refused {
        detail: "destination busy".to_string(),
    });
    check!(matches!(error, JobError::Terminal { .. }));

    let error = JobError::from(StoreError::Invariant {
        detail: "row id missing".to_string(),
    });
    check!(matches!(error, JobError::Terminal { .. }));
    Ok(())
}

#[test]
fn environmental_store_errors_classify_transient() {
    let error = JobError::from(StoreError::Io {
        path: PathBuf::from("/tmp/test"),
        source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
    });
    assert!(matches!(error, JobError::Transient { .. }));
}

#[test]
fn identical_semantic_requests_with_same_generation_attach() {
    let key_a = run_key("report", &["core"], DEFAULT_GENERATION);
    let key_b = run_key("report", &["core"], DEFAULT_GENERATION);
    assert_eq!(
        key_a, key_b,
        "identical requests with same generation must share a key"
    );
    assert_eq!(key_a, "report:core:1");

    let key_c = run_key("bests", &["all", "2027", "50"], DEFAULT_GENERATION);
    let key_d = run_key("bests", &["all", "2027", "50"], DEFAULT_GENERATION);
    assert_eq!(key_c, key_d);
    assert_eq!(key_c, "bests:all:2027:50:1");

    let key_e = run_key("consolidate", &[], DEFAULT_GENERATION);
    let key_f = run_key("consolidate", &[], DEFAULT_GENERATION);
    assert_eq!(key_e, key_f);
    assert_eq!(key_e, "consolidate:1");
}

#[test]
fn same_semantics_different_generation_produces_new_key() {
    let key_default = run_key("report", &["core"], DEFAULT_GENERATION);
    let key_new = run_key("report", &["core"], "2");
    assert_ne!(
        key_default, key_new,
        "same semantic parts with different generation must produce different keys"
    );
    assert_eq!(key_default, "report:core:1");
    assert_eq!(key_new, "report:core:2");

    assert_ne!(
        run_key("bests", &["all", "2027", "50"], "1"),
        run_key("bests", &["all", "2027", "50"], "abc-def"),
        "any two generation values must produce different keys"
    );
}

#[test]
fn differing_semantic_parts_produce_different_keys() {
    assert_ne!(
        run_key("report", &["core"], DEFAULT_GENERATION),
        run_key("report", &["all_sources"], DEFAULT_GENERATION),
        "different scope must produce different keys"
    );

    assert_ne!(
        run_key("bests", &["all", "2027", "50"], DEFAULT_GENERATION),
        run_key("bests", &["all", "2027", "100"], DEFAULT_GENERATION),
        "different limit must produce different keys"
    );

    assert_ne!(
        run_key(
            "workbook",
            &["2027", "core", "all", "."],
            DEFAULT_GENERATION
        ),
        run_key(
            "workbook",
            &["2028", "core", "all", "."],
            DEFAULT_GENERATION
        ),
        "different grad year must produce different keys"
    );

    assert_ne!(
        run_key(
            "workbook",
            &["2027", "core", "all", "."],
            DEFAULT_GENERATION
        ),
        run_key(
            "workbook",
            &["2027", "core", "all", "/tmp/fresh"],
            DEFAULT_GENERATION
        ),
        "different out path must produce different keys"
    );

    assert_ne!(
        run_key("workbook", &["2027", "core", "50", "."], DEFAULT_GENERATION),
        run_key(
            "workbook",
            &["2027", "core", "100", "."],
            DEFAULT_GENERATION
        ),
        "different limit must produce different keys"
    );
}

#[test]
fn run_key_is_independent_of_wall_clock() {
    let key = run_key("report", &["core"], DEFAULT_GENERATION);
    let segments: Vec<&str> = key.split(':').collect();
    assert_eq!(segments.len(), 3, "report:core:1 has exactly 3 segments");
    assert_eq!(
        segments[2], "1",
        "the last segment must be the generation, not a timestamp"
    );
    assert_eq!(segments[0], "report");
    assert_eq!(segments[1], "core");

    let key2 = run_key("bests", &["all", "2027", "50"], "abc");
    let segments2: Vec<&str> = key2.split(':').collect();
    assert_eq!(
        segments2.len(),
        5,
        "bests:all:2027:50:abc has exactly 5 segments"
    );
    assert_eq!(
        segments2[4], "abc",
        "the last segment must be the generation"
    );

    let key3 = run_key("consolidate", &[], DEFAULT_GENERATION);
    let segments3: Vec<&str> = key3.split(':').collect();
    assert_eq!(segments3.len(), 2, "consolidate:1 has 2 segments");
    assert_eq!(segments3[0], "consolidate");
    assert_eq!(segments3[1], "1");
}

#[test]
fn classification_survives_through_job_error() {
    use super::job_error;

    let terminal = job_error(JobError::Terminal {
        message: "robots.txt disallowed".to_string(),
    });
    assert!(
        format!("{terminal:?}").contains("Terminal"),
        "JobError::Terminal must produce TerminalHandlerError, got {terminal:?}"
    );

    let transient = job_error(JobError::Transient {
        message: "transport error".to_string(),
    });
    assert!(
        format!("{transient:?}").contains("Transient"),
        "JobError::Transient must produce TransientFailure, got {transient:?}"
    );
}

#[test]
fn crawl_error_survives_through_collect_error_and_job_error() {
    use super::job_error;

    let browser_not_retryable_h =
        job_error(collect_error(CrawlError::Fetch(FetchError::BrowserLane {
            url: "https://example.com".to_string(),
            detail: "human_required".to_string(),
            retryable: false,
        })));
    assert!(
        format!("{browser_not_retryable_h:?}").contains("Terminal"),
        "BrowserLane{{retryable:false}} must become Terminal HandlerError, got {browser_not_retryable_h:?}"
    );

    let timeout_h = job_error(collect_error(CrawlError::Fetch(FetchError::Timeout {
        url: "https://example.com".to_string(),
        timeout_secs: 30,
    })));
    assert!(
        format!("{timeout_h:?}").contains("Transient"),
        "Timeout must become Transient HandlerError, got {timeout_h:?}"
    );

    let schema_h = job_error(collect_error(CrawlError::Schema {
        url: "https://example.com".to_string(),
        detail: "missing required field".to_string(),
    }));
    assert!(
        format!("{schema_h:?}").contains("Terminal"),
        "Schema mismatch must become Terminal HandlerError, got {schema_h:?}"
    );
}
