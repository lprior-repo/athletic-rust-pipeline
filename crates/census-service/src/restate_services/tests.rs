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

#[test]
fn table_names_resolve_and_reject_typos() {
    assert_eq!(resolve_table("schools").unwrap(), Table::Schools);
    assert_eq!(resolve_table("performances").unwrap(), Table::Performances);
    assert!(resolve_table("school").is_err());
    assert!(resolve_table("").is_err());
}

#[test]
fn empty_table_list_means_every_table_in_order() {
    assert_eq!(resolve_tables(&[]).unwrap(), Table::ALL.to_vec());
    let requested = vec!["meets".to_string(), "meets".to_string()];
    assert_eq!(resolve_tables(&requested).unwrap(), vec![Table::Meets]);
}

#[test]
fn an_omitted_scope_matches_the_cli_default_and_unknown_scopes_are_rejected() {
    assert_eq!(resolve_scope(None).unwrap(), Scope::AllSources);
    assert_eq!(resolve_scope(Some("core")).unwrap(), Scope::Core);
    assert_eq!(
        resolve_scope(Some("all_sources")).unwrap(),
        Scope::AllSources
    );
    assert!(resolve_scope(Some("all")).is_err());
}

#[test]
fn cohort_label_names_the_reduction() {
    assert_eq!(cohort_label(Some(2027)), "co2027");
    assert_eq!(cohort_label(None), "all");
}

fn physical(store: &Store, table: Table) -> (u64, u64) {
    (
        store.walk_table(table).unwrap().rows,
        store.receipt_count().unwrap(),
    )
}

#[test]
fn three_attempts_at_one_operation_append_it_once_and_leave_one_receipt() {
    let dir = tempfile::tempdir().unwrap();
    let operation = "wiaa_results_wi:inv-1:2026-W39:performances:0:0";
    let rows = vec![
        serde_json::json!({"id": "perf:wi:1", "mark": "10.94"}),
        serde_json::json!({"id": "perf:wi:2", "mark": "11.02"}),
    ];
    let digest = payload_digest(Table::Performances, &rows).unwrap();

    let store = Store::open(dir.path()).unwrap();
    let first = apply_observations(&store, Table::Performances, &rows, operation, &digest).unwrap();
    assert_eq!(
        first.appended(),
        rows.len() as u64,
        "the first attempt appends the page: {first:?}"
    );
    let after_first = physical(&store, Table::Performances);
    assert_eq!(after_first, (2, 1), "two rows, one receipt");
    drop(store);

    let store = Store::open(dir.path()).unwrap();
    for attempt in 2..=3 {
        let again = apply_observations(&store, Table::Performances, &rows, operation, &digest)
            .unwrap()
            .appended();
        assert_eq!(again, 0, "attempt {attempt} appended nothing");
        assert_eq!(
            physical(&store, Table::Performances),
            after_first,
            "attempt {attempt} left the store exactly as attempt 1 did"
        );
    }

    let receipt = store.receipt(operation).unwrap().unwrap();
    assert_eq!(
        receipt.appended, 2,
        "the receipt describes the one application"
    );
    assert_eq!(receipt.digest, digest);
}

#[test]
fn a_second_operation_with_different_rows_is_not_mistaken_for_a_replay() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let rows = vec![serde_json::json!({"id": "perf:wi:1", "mark": "10.94"})];
    let digest = payload_digest(Table::Performances, &rows).unwrap();

    let first = apply_observations(&store, Table::Performances, &rows, "op-1", &digest).unwrap();
    assert_eq!(first.appended(), 1);
    let second = apply_observations(&store, Table::Performances, &rows, "op-2", &digest).unwrap();
    assert_eq!(
        second.appended(),
        1,
        "a different operation appends its own page, overlapping rows and all"
    );
    assert_eq!(physical(&store, Table::Performances), (2, 2));
}

#[test]
fn rows_without_an_id_are_rejected_by_the_store_and_nothing_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let rows = vec![serde_json::json!({"name": "no id here"})];
    assert!(apply_observations(&store, Table::Schools, &rows, "op-1", "digest-1").is_err());
    assert_eq!(store.stats().unwrap().observations, 0);
    assert_eq!(
        store.receipt_count().unwrap(),
        0,
        "a rejected page leaves no receipt behind"
    );

    let good = vec![serde_json::json!({"id": "school:wi:test", "name": "Test"})];
    assert_eq!(
        apply_observations(&store, Table::Schools, &good, "op-2", "digest-2")
            .unwrap()
            .appended(),
        1
    );
    assert_eq!(store.stats().unwrap().observations, 1);
    assert_eq!(
        apply_observations(&store, Table::Schools, &good, "op-2", "digest-2")
            .unwrap()
            .appended(),
        0,
        "the same operation twice appends its page once"
    );
    assert_eq!(store.stats().unwrap().observations, 1);
}

#[test]
fn oversized_batches_are_refused_without_touching_the_store() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let rows = vec![serde_json::json!({"id": "x"}); MAX_ROWS_PER_REQUEST + 1];
    let refused = apply_observations(&store, Table::Schools, &rows, "op-1", "digest-1")
        .map_err(JobError::from)
        .expect_err("a batch over the ceiling is refused");
    assert!(matches!(refused, JobError::Terminal { .. }));
    assert_eq!(store.stats().unwrap().observations, 0);
    assert_eq!(
        store.receipt_count().unwrap(),
        0,
        "the refusal happens before the commit, so no receipt is written"
    );
}

#[test]
fn job_failures_classify_for_retry_but_a_violated_invariant_and_a_panic_never_retry() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let region = Arc::new(Spawner::new());

    let refused = runtime.block_on(blocking(Arc::clone(&region), || {
        Err::<u8, StoreError>(StoreError::Invariant {
            detail: "50001 rows exceeds the per-request ceiling of 50000".to_string(),
        })
    }));
    assert!(matches!(refused, Err(JobError::Terminal { .. })));

    let transient = runtime.block_on(blocking(Arc::clone(&region), || {
        Err::<u8, StoreError>(StoreError::Io {
            path: PathBuf::from("/dev/null/nowhere"),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        })
    }));
    assert!(matches!(transient, Err(JobError::Transient { .. })));

    let report = runtime.block_on(blocking(Arc::clone(&region), || {
        Err::<u8, ReportError>(ReportError::Store(StoreError::Invariant {
            detail: "table schools would exceed 20000000 rows in one scan".to_string(),
        }))
    }));
    assert!(matches!(report, Err(JobError::Terminal { .. })));

    let report_invariant = runtime.block_on(blocking(Arc::clone(&region), || {
        Err::<u8, ReportError>(ReportError::Invariant {
            detail: "the core scope reports more than the all-sources scope".to_string(),
        })
    }));
    assert!(matches!(report_invariant, Err(JobError::Terminal { .. })));

    let report_io = runtime.block_on(blocking(Arc::clone(&region), || {
        Err::<u8, ReportError>(ReportError::Io {
            path: PathBuf::from("/dev/null/nowhere"),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        })
    }));
    assert!(matches!(report_io, Err(JobError::Transient { .. })));

    let panicked = runtime.block_on(blocking(Arc::clone(&region), || -> Result<u8, JobError> {
        panic!("boom");
    }));
    assert!(matches!(panicked, Err(JobError::Terminal { .. })));
}

fn national_request(jurisdictions: Vec<UsJurisdiction>) -> NationalRequest {
    NationalRequest {
        season: SchoolYear::new(2026).expect("2026 is a season"),
        revision: Revision(1),
        jurisdictions,
        refresh: false,
        limit_per_state: None,
        concurrency: 4,
        observed_on: None,
        authorized_hosts: Vec::new(),
        source_parallelism: census_crawl::net::DEFAULT_FAMILY_PARALLELISM,
    }
}

#[test]
fn an_empty_jurisdiction_list_covers_the_census_scope_in_declaration_order() {
    let targets = national::targets(&national_request(Vec::new())).unwrap();
    assert_eq!(targets.len(), UsJurisdiction::CENSUS_SCOPE.len());
    let jurisdictions: Vec<UsJurisdiction> = targets.iter().map(|row| row.0).collect();
    assert_eq!(jurisdictions, UsJurisdiction::CENSUS_SCOPE.to_vec());
    for (jurisdiction, key) in &targets {
        assert_eq!(
            key.as_str(),
            WorkflowIdentity::jurisdiction(
                *jurisdiction,
                SchoolYear::new(2026).expect("2026 is a season"),
                Revision(1),
            )
            .as_str()
        );
    }
}

#[test]
fn a_named_jurisdiction_set_keeps_the_callers_order() {
    let targets = national::targets(&national_request(vec![
        UsJurisdiction::Iowa,
        UsJurisdiction::Wisconsin,
    ]))
    .unwrap();
    assert_eq!(targets.len(), 2);
    assert_eq!(targets[0].0, UsJurisdiction::Iowa);
    assert_eq!(targets[0].1, "jurisdiction:IA:2026-27:1");
    assert_eq!(targets[1].0, UsJurisdiction::Wisconsin);
    assert_eq!(targets[1].1, "jurisdiction:WI:2026-27:1");
}

#[test]
fn a_state_named_twice_is_refused_instead_of_walked_twice() {
    let error = national::targets(&national_request(vec![
        UsJurisdiction::Wisconsin,
        UsJurisdiction::Iowa,
        UsJurisdiction::Wisconsin,
    ]))
    .unwrap_err();
    assert!(
        format!("{error:?}").contains("WI"),
        "the refusal must name the repeated state: {error:?}"
    );
}

#[test]
fn the_walk_options_carry_the_runs_shared_knobs() {
    let mut request = national_request(vec![UsJurisdiction::Iowa]);
    request.limit_per_state = Some(17);
    request.concurrency = 3;
    request.refresh = true;
    let projected = request.for_jurisdiction(UsJurisdiction::Iowa);
    let options = super::options_for_request(&projected, "2026-09-21").unwrap();
    assert_eq!(options.jurisdictions, vec![UsJurisdiction::Iowa]);
    assert_eq!(options.limit_per_state, Some(17));
    assert_eq!(options.concurrency, 3);
    assert_eq!(options.state_concurrency, 1);
    assert!(options.refresh);
    assert_eq!(
        options.school_year,
        SchoolYear::new(2026).expect("2026 is a season")
    );
    assert_eq!(options.observed_on, "2026-09-21");
}

#[test]
fn the_collection_date_is_the_days_today_unless_the_request_names_one() {
    let request = national_request(vec![UsJurisdiction::Iowa]);
    let today = super::options_for_request(
        &request.for_jurisdiction(UsJurisdiction::Iowa),
        "2026-09-21",
    )
    .unwrap();
    assert_eq!(today.observed_on, "2026-09-21");

    let mut dated = request;
    dated.observed_on = Some("2026-09-01".to_string());
    let named =
        super::options_for_request(&dated.for_jurisdiction(UsJurisdiction::Iowa), "2026-09-21")
            .unwrap();
    assert_eq!(named.observed_on, "2026-09-01");
}

#[test]
fn the_roster_ceiling_admits_its_boundary_and_refuses_one_past_it() {
    let mut request = national_request(vec![UsJurisdiction::Iowa]);
    request.limit_per_state = Some(MAX_LIMIT_PER_STATE);
    let projected = request.for_jurisdiction(UsJurisdiction::Iowa);
    assert_eq!(
        super::options_for_request(&projected, "2026-09-21")
            .unwrap()
            .limit_per_state,
        Some(MAX_LIMIT_PER_STATE)
    );

    let mut over = national_request(vec![UsJurisdiction::Iowa]);
    over.limit_per_state = Some(MAX_LIMIT_PER_STATE.saturating_add(1));
    let projected = over.for_jurisdiction(UsJurisdiction::Iowa);
    let error = super::options_for_request(&projected, "2026-09-21").unwrap_err();
    assert!(
        format!("{error:?}").contains("limit_per_state"),
        "the refusal must name the knob: {error:?}"
    );
}

#[test]
fn a_walk_with_no_concurrency_is_refused() {
    let mut request = national_request(vec![UsJurisdiction::Iowa]);
    request.concurrency = 0;
    let projected = request.for_jurisdiction(UsJurisdiction::Iowa);
    let error = super::options_for_request(&projected, "2026-09-21").unwrap_err();
    assert!(
        format!("{error:?}").contains("concurrency"),
        "the refusal must name the knob: {error:?}"
    );
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
fn a_state_that_did_not_answer_becomes_a_failure_row_and_the_run_keeps_its_summaries() {
    let key = "jurisdiction:IA:2026-27:1";
    let failed = national::classify(
        UsJurisdiction::Iowa,
        key,
        Err(TerminalError::new("index host refused the walk")),
    );
    let national::Completion::Unanswered(failure) = failed else {
        panic!("a failed call must classify as a failure row, not a summary");
    };
    assert_eq!(failure.jurisdiction, UsJurisdiction::Iowa);
    assert_eq!(
        failure.identity, key,
        "the row must name the identity the run addressed"
    );
    assert!(
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
        panic!("a returned report must classify as a summary");
    };
    assert_eq!(summary.jurisdiction, UsJurisdiction::Wisconsin);
    assert_eq!(summary.identity, "jurisdiction:WI:2026-27:1");
    assert_eq!(summary.rosters_total, 7);
    assert_eq!(summary.rosters_committed, 5);
    assert_eq!(summary.rosters_skipped, 0);
    assert_eq!(summary.class_of_2027, 3);
    assert_eq!(
        summary.rosters_remaining, 2,
        "seven teams with five rosters walked leave two remaining"
    );
    assert_eq!(summary.athletes, 11);
    assert!(!summary.blocked, "a completed walk is not a blocked one");
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
fn fetch_error_nonretryable_variants_become_terminal() {
    use census_crawl::net::FetchError;

    let too_large = collect_error(CrawlError::Fetch(FetchError::TooLarge {
        url: "https://example.com".to_string(),
    }));
    assert!(
        matches!(too_large, JobError::Terminal { .. }),
        "TooLarge must be terminal"
    );

    let browser_not_retryable = collect_error(CrawlError::Fetch(FetchError::BrowserLane {
        url: "https://example.com".to_string(),
        detail: "human_required".to_string(),
        retryable: false,
    }));
    assert!(
        matches!(browser_not_retryable, JobError::Terminal { .. }),
        "BrowserLane {{ retryable: false }} must be terminal (the defect was it became Transient)"
    );

    let invalid_url = collect_error(CrawlError::Fetch(FetchError::InvalidUrl {
        url: "not a url".to_string(),
        source: url::Url::parse("not a url").unwrap_err(),
    }));
    assert!(matches!(invalid_url, JobError::Terminal { .. }));

    let http_404 = collect_error(CrawlError::Fetch(FetchError::Http {
        status: 404,
        url: "https://example.com".to_string(),
    }));
    assert!(matches!(http_404, JobError::Terminal { .. }));

    let http_403 = collect_error(CrawlError::Fetch(FetchError::Http {
        status: 403,
        url: "https://example.com".to_string(),
    }));
    assert!(matches!(http_403, JobError::Terminal { .. }));
}

#[test]
fn non_fetch_crawl_errors_are_terminal() {
    let schema = collect_error(CrawlError::Schema {
        url: "https://example.com".to_string(),
        detail: "missing field".to_string(),
    });
    assert!(matches!(schema, JobError::Terminal { .. }));

    let decode = collect_error(CrawlError::Decode {
        url: "https://example.com".to_string(),
        source: serde_json::from_str::<serde_json::Value>("not json").unwrap_err(),
    });
    assert!(matches!(decode, JobError::Terminal { .. }));

    let domain = collect_error(CrawlError::Domain(census_domain::DomainError::OutOfRange {
        field: "grad_year",
    }));
    assert!(matches!(domain, JobError::Terminal { .. }));

    let arithmetic = collect_error(CrawlError::Arithmetic {
        detail: "overflow".to_string(),
    });
    assert!(matches!(arithmetic, JobError::Terminal { .. }));

    let io_err = collect_error(CrawlError::Io {
        path: std::path::PathBuf::from("/dev/null/nowhere"),
        source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
    });
    assert!(matches!(io_err, JobError::Terminal { .. }));

    let encode = collect_error(CrawlError::Encode {
        table: "schools".to_string(),
        source: serde_json::Error::io(std::io::Error::from(std::io::ErrorKind::Other)),
    });
    assert!(matches!(encode, JobError::Terminal { .. }));
}

#[test]
fn deterministic_store_errors_classify_terminal() {
    let error = JobError::from(StoreError::CounterOverflow);
    assert!(matches!(error, JobError::Terminal { .. }));

    let error = JobError::from(StoreError::Decode {
        key: "schools:1".to_string(),
        source: serde_json::from_str::<serde_json::Value>("not json").unwrap_err(),
    });
    assert!(matches!(error, JobError::Terminal { .. }));

    let error = JobError::from(StoreError::Json {
        detail: "raw row".to_string(),
        source: serde_json::from_str::<serde_json::Value>("not json").unwrap_err(),
    });
    assert!(matches!(error, JobError::Terminal { .. }));

    let error = JobError::from(StoreError::SnapshotRow {
        path: PathBuf::from("/tmp/out/schools.jsonl"),
        line: 42,
        source: serde_json::from_str::<serde_json::Value>("not json").unwrap_err(),
    });
    assert!(matches!(error, JobError::Terminal { .. }));

    let error = JobError::from(StoreError::TooManyRows {
        table: "schools".to_string(),
        max: 20_000_000,
    });
    assert!(matches!(error, JobError::Terminal { .. }));

    let error = JobError::from(StoreError::JournalTooLarge {
        what: "value",
        phase: "teams".to_string(),
        key: "wi:1".to_string(),
        bytes: 1_000_000,
        max: 500_000,
    });
    assert!(matches!(error, JobError::Terminal { .. }));

    let error = JobError::from(StoreError::Refused {
        detail: "destination busy".to_string(),
    });
    assert!(matches!(error, JobError::Terminal { .. }));

    let error = JobError::from(StoreError::Invariant {
        detail: "row id missing".to_string(),
    });
    assert!(matches!(error, JobError::Terminal { .. }));
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
