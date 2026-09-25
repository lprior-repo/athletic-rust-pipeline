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
    // `census-service report` without `--core` reports every source, so the service must too.
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

/// Physical rows and receipts a store holds: counted from the keys the keyspaces carry, not from a
/// row ledger a writer maintains for itself.
fn physical(store: &Store, table: Table) -> (u64, u64) {
    (
        store.walk_table(table).unwrap().rows,
        store.receipt_count().unwrap(),
    )
}

#[test]
fn three_attempts_at_one_operation_append_it_once_and_leave_one_receipt() {
    // The window §15 names: the append commits, the writer dies before it hears its own
    // acknowledgement, and the same unit of work is offered again — by the retry policy, and again
    // after a restart. Three attempts, one page: the second and third must find the store's receipt
    // and append nothing, and the physical counts after all three must be the counts after the
    // first.
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
    // No flush, no close: the commit is the durability boundary, so what a crash leaves is this.
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
    // The other half of the contract: an id the store has not seen is new work, even beside a page
    // whose rows overlap the first operation's. Only the operation id decides, and the digest is
    // checked under it.
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
    // The ceiling is an admission bound, not a hiccup: it must reach Restate as a terminal outcome,
    // or the invocation would replay a batch that can never fit.
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

    // The store's own bound was violated: replaying the same journal value cannot restore it.
    let refused = runtime.block_on(blocking(Arc::clone(&region), || {
        Err::<u8, StoreError>(StoreError::Invariant {
            detail: "50001 rows exceeds the per-request ceiling of 50000".to_string(),
        })
    }));
    assert!(matches!(refused, Err(JobError::Terminal { .. })));

    // Every other store failure is what a journaled retry repairs.
    let transient = runtime.block_on(blocking(Arc::clone(&region), || {
        Err::<u8, StoreError>(StoreError::Io {
            path: PathBuf::from("/dev/null/nowhere"),
            source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        })
    }));
    assert!(matches!(transient, Err(JobError::Transient { .. })));

    // A report failure classifies through its own conversion, including the store failure it wraps.
    let report = runtime.block_on(blocking(Arc::clone(&region), || {
        Err::<u8, ReportError>(ReportError::Store(StoreError::Invariant {
            detail: "table schools would exceed 20000000 rows in one scan".to_string(),
        }))
    }));
    assert!(matches!(report, Err(JobError::Terminal { .. })));

    // A report-side invariant is terminal for the same reason a store-side one is: it is a bug, not
    // a bad night, and the replay would reproduce it exactly.
    let report_invariant = runtime.block_on(blocking(Arc::clone(&region), || {
        Err::<u8, ReportError>(ReportError::Invariant {
            detail: "the core scope reports more than the all-sources scope".to_string(),
        })
    }));
    assert!(matches!(report_invariant, Err(JobError::Terminal { .. })));

    // Everything else the report path can fail with stays retryable.
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

// ------------------------------------------------------- national fan-out admission

/// The national request every fan-out test varies: season 2026-27 at revision 1, the CLI's default
/// concurrency, and no roster ceiling.
fn national_request(jurisdictions: Vec<UsJurisdiction>) -> NationalRequest {
    NationalRequest {
        season: SchoolYear::new(2026).expect("2026 is a season"),
        revision: Revision(1),
        jurisdictions,
        refresh: false,
        limit_per_state: None,
        concurrency: 4,
        observed_on: None,
    }
}

#[test]
fn an_empty_jurisdiction_list_covers_the_census_scope_in_declaration_order() {
    let targets = national::targets(&national_request(Vec::new())).unwrap();
    assert_eq!(targets.len(), UsJurisdiction::CENSUS_SCOPE.len());
    let jurisdictions: Vec<UsJurisdiction> = targets.iter().map(|row| row.0).collect();
    assert_eq!(jurisdictions, UsJurisdiction::CENSUS_SCOPE.to_vec());
    // Each state is addressed by the identity of its own census, not by the position it was pushed.
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
    // One jurisdiction is one state host: nothing to interleave, whatever the national run asked for.
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

// ------------------------------------------------------ national fan-out tolerance

/// One jurisdiction's report as `JurisdictionCensus` returns it: seven teams, five rosters walked,
/// eleven athletes of whom three are in the 2027 cohort — enough to see the summary's own arithmetic.
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
            rosters_done: 5,
            rosters_skipped: 0,
            athletes: 11,
            class_of_2027: 3,
            class_of_2027_boys: 2,
            class_of_2027_girls: 1,
            empty_rosters: 1,
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

/// §69: a state that could not be walked is a *row* in the national report, not an abort, and one
/// state's outage must not cost the fan-out the states that did answer. This is the whole of that
/// decision, so it is tested without a Restate context.
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

    // The same fold takes the summary of a state that did answer, with the owed rosters the report
    // publishes derived from the two counts the jurisdiction returned.
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
    assert_eq!(summary.teams, 7);
    assert_eq!(summary.rosters_done, 5);
    assert_eq!(summary.rosters_skipped, 0);
    assert_eq!(summary.class_of_2027, 3);
    assert_eq!(
        summary.rosters_owed,
        Some(2),
        "seven teams with five rosters walked leave two owed"
    );
    assert_eq!(summary.athletes, 11);
    assert_eq!(
        summary.blocked,
        Some(false),
        "a completed walk is not a blocked one"
    );
}

/// A surface that only renders under a browser session, as a literal descriptor.
///
/// Synthetic on purpose: no registry entry declares `TransportKind::Browser` today (the only
/// mentions are `sources/registry.rs:58` and the derivation at `:179`), so the real applicability
/// table cannot reach this branch at all. The origin is deliberately not the artifact placeholder,
/// because `access_class()` reads that first and would classify the descriptor as `Artifact`.
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

/// A source this machine cannot run is owed, not gone: the refusal is evidence, and it never
/// reaches the path the invocation retry owns.
///
/// Three claims, in the order the run depends on them. The refusal names the source, the acquisition
/// it would have taken and what is missing. It is not one of the units a dispatcher may fetch — and
/// the fetch path is the only place a `CrawlError` arises, so it is the only thing
/// `JobError::Transient` (the invocation retry's one feed, `support.rs:20`) can be built from, which
/// is what keeps a missing lane out of the three-attempt ceiling at `census.rs:46`. And the record it
/// leaves is the real one: a source object that has accepted nothing, counted by
/// `owed_source_objects` — the same function `open_work.rs` folds into the seal's `source_objects`,
/// so while the refusal stands the census cannot seal.
///
/// The lane-configured plan is the control: the same source becomes an ordinary sweepable unit and
/// nothing is owed, so what changed is this machine, not the source.
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
// ----------------------------------------------------------------------- defect regression

/// FetchError variants that retryable() says are retryable: all become Transient.
///
/// The transport classified these as transient, and the census honours that verdict.
/// Transport cannot be constructed without a reqwest::Error, so we verify retryable()
/// returns true and confirm the other variants' classification through collect_error.
#[test]
fn fetch_error_retryable_variants_become_transient() {
    use census_crawl::net::FetchError;

    // Transport is retryable (verified by retryable() in census-crawl; constructible only with
    // reqwest::Error, so we skip direct construction here). The collect_error path below
    // confirms the other retryable variants.

    // Timeout is retryable.
    assert!(FetchError::Timeout {
        url: "https://example.com".to_string(),
        timeout_secs: 30,
    }
    .retryable());

    // RateLimited is retryable.
    assert!(FetchError::RateLimited {
        url: "https://example.com".to_string(),
        retry_after_secs: None,
    }
    .retryable());

    // 5xx HTTP responses are retryable.
    assert!(FetchError::Http {
        status: 500,
        url: "https://example.com".to_string(),
    }
    .retryable());

    // 429 is retryable.
    assert!(FetchError::Http {
        status: 429,
        url: "https://example.com".to_string(),
    }
    .retryable());

    // BrowserLane { retryable: true } is retryable.
    assert!(FetchError::BrowserLane {
        url: "https://example.com".to_string(),
        detail: "profile busy".to_string(),
        retryable: true,
    }
    .retryable());

    // Now confirm collect_error honours these through the actual classification path.
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

/// FetchError variants that retryable() says are NOT retryable: all become Terminal.
///
/// Robots, TooLarge, BrowserLane { retryable: false }, InvalidUrl, Decode, Encode, Client,
/// and Invariant are deterministic — the same input reproduces the same failure, so retry
/// cannot help. The defect was that these fell through to Transient.
#[test]
fn fetch_error_nonretryable_variants_become_terminal() {
    use census_crawl::net::FetchError;

    let robots = collect_error(CrawlError::Fetch(FetchError::Robots(
        "https://example.com".to_string(),
    )));
    assert!(
        matches!(robots, JobError::Terminal { .. }),
        "Robots must be terminal (the defect was it became Transient)"
    );

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

    // 4xx HTTP (non-429) is terminal.
    let http_404 = collect_error(CrawlError::Fetch(FetchError::Http {
        status: 404,
        url: "https://example.com".to_string(),
    }));
    assert!(matches!(http_404, JobError::Terminal { .. }));

    // 4xx other non-429 is terminal.
    let http_403 = collect_error(CrawlError::Fetch(FetchError::Http {
        status: 403,
        url: "https://example.com".to_string(),
    }));
    assert!(matches!(http_403, JobError::Terminal { .. }));
}

/// Non-fetch CrawlError variants: Schema, Decode, Domain, Arithmetic, Io, RegexInit are all
/// terminal because reading the same bytes again does not make them less wrong.
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

/// Every deterministic StoreError classifies Terminal; only environmental ones are Transient.
///
/// The defect was that everything except Invariant was transient. CounterOverflow, JournalTooLarge,
/// Decode, Json, SnapshotRow, TooManyRows, Refused, and Legacy are deterministic — retrying with
/// the same input cannot fix them.
#[test]
fn deterministic_store_errors_classify_terminal() {
    // CounterOverflow: no more u64 available, retry yields nothing.
    let error = JobError::from(StoreError::CounterOverflow);
    assert!(matches!(error, JobError::Terminal { .. }));

    // Decode: corrupt JSON in store, retry reproduces the same corruption.
    let error = JobError::from(StoreError::Decode {
        key: "schools:1".to_string(),
        source: serde_json::from_str::<serde_json::Value>("not json").unwrap_err(),
    });
    assert!(matches!(error, JobError::Terminal { .. }));

    // Json: unkeyed bytes failed to decode.
    let error = JobError::from(StoreError::Json {
        detail: "raw row".to_string(),
        source: serde_json::from_str::<serde_json::Value>("not json").unwrap_err(),
    });
    assert!(matches!(error, JobError::Terminal { .. }));

    // SnapshotRow: a JSONL snapshot line is corrupt.
    let error = JobError::from(StoreError::SnapshotRow {
        path: PathBuf::from("/tmp/out/schools.jsonl"),
        line: 42,
        source: serde_json::from_str::<serde_json::Value>("not json").unwrap_err(),
    });
    assert!(matches!(error, JobError::Terminal { .. }));

    // TooManyRows: a scan would exceed the ceiling — retry does not shrink the table.
    let error = JobError::from(StoreError::TooManyRows {
        table: "schools".to_string(),
        max: 20_000_000,
    });
    assert!(matches!(error, JobError::Terminal { .. }));

    // JournalTooLarge: entry exceeds the ceiling — the data is what it is.
    let error = JobError::from(StoreError::JournalTooLarge {
        what: "value",
        phase: "teams".to_string(),
        key: "wi:1".to_string(),
        bytes: 1_000_000,
        max: 500_000,
    });
    assert!(matches!(error, JobError::Terminal { .. }));

    // Refused: the request did not meet the condition the store enforces.
    let error = JobError::from(StoreError::Refused {
        detail: "destination busy".to_string(),
    });
    assert!(matches!(error, JobError::Terminal { .. }));

    // Legacy: the one-time import failed — retrying the same data fails again.
    let error = JobError::from(StoreError::Legacy {
        detail: "column count mismatch".to_string(),
    });
    assert!(matches!(error, JobError::Terminal { .. }));

    // Invariant: replaying cannot restore it (this was already terminal, confirming it stays so).
    let error = JobError::from(StoreError::Invariant {
        detail: "row id missing".to_string(),
    });
    assert!(matches!(error, JobError::Terminal { .. }));
}

/// Only environmental StoreError variants are Transient.
///
/// Open, Flush, Read, Write all wrap `fjall::Error` and share the same transient classification;
/// Io wraps `std::io::Error`. Testing Io covers the pattern — the match arm for all five is
/// identical: `Self::Transient { message }`.
#[test]
fn environmental_store_errors_classify_transient() {
    // Io: sidecar file I/O might resolve on retry.
    let error = JobError::from(StoreError::Io {
        path: PathBuf::from("/tmp/test"),
        source: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
    });
    assert!(matches!(error, JobError::Transient { .. }));

    // The other environmental variants (Open, Flush, Read, Write) all carry fjall::Error
    // as their source and are classified identically — the match arm is the same for all.
}

/// run_key: identical semantic requests with the same generation produce identical keys.
///
/// Two calls with the same job, parts, and generation must produce the same key, so a rerun
/// attaches to the existing workflow.
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

/// run_key: same semantic parts but different generation produces a different key.
///
/// This is the disambiguation guarantee: an operator who specifies a new `--generation` (or
/// `--run-id`) can start a fresh run even when the same request is already completed.
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

    // Different generation values must always differ.
    assert_ne!(
        run_key("bests", &["all", "2027", "50"], "1"),
        run_key("bests", &["all", "2027", "50"], "abc-def"),
        "any two generation values must produce different keys"
    );
}

/// run_key: differing semantic parts produce different keys regardless of generation.
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

    // Different out paths produce different keys.
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

    // Different limits produce different keys.
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

/// run_key is independent of wall-clock time and always ends with the generation.
///
/// The key shape is `<job>:<parts>:<generation>`. The last segment is always the generation,
/// not a timestamp.
#[test]
fn run_key_is_independent_of_wall_clock() {
    // Verify the key shape: for a key with N parts, there are N+2 segments
    // (job + parts + generation).
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

    // For an empty-parts key, there are exactly 2 segments (job + generation).
    let key3 = run_key("consolidate", &[], DEFAULT_GENERATION);
    let segments3: Vec<&str> = key3.split(':').collect();
    assert_eq!(segments3.len(), 2, "consolidate:1 has 2 segments");
    assert_eq!(segments3[0], "consolidate");
    assert_eq!(segments3[1], "1");
}

/// The disposition at the boundary: a deterministic failure reaches the handler as a terminal
/// error and a transient failure stays retryable. The old code used `?` directly, which let the
/// SDK's blanket From<StdError> convert JobError::Terminal to a retryable HandlerError.
#[test]
fn classification_survives_through_job_error() {
    use super::job_error;

    // Terminal CrawlError must survive as Terminal HandlerError.
    let terminal = job_error(JobError::Terminal {
        message: "robots.txt disallowed".to_string(),
    });
    // The HandlerError must be terminal, not transient.
    assert!(
        format!("{terminal:?}").contains("Terminal"),
        "JobError::Terminal must produce TerminalHandlerError, got {terminal:?}"
    );

    // Transient CrawlError must survive as Transient HandlerError.
    let transient = job_error(JobError::Transient {
        message: "transport error".to_string(),
    });
    assert!(
        format!("{transient:?}").contains("Transient"),
        "JobError::Transient must produce TransientFailure, got {transient:?}"
    );
}

/// End-to-end: CrawlError flows through collect_error + job_error to HandlerError with correct
/// classification. This is the boundary test the handler depends on — the old `?` path lost
/// terminal classification through the SDK's blanket From<StdError>.
#[test]
fn crawl_error_survives_through_collect_error_and_job_error() {
    use super::job_error;

    // Robots (non-retryable FetchError) must become Terminal HandlerError.
    let robots_h = job_error(collect_error(CrawlError::Fetch(FetchError::Robots(
        "https://example.com/robots.txt".to_string(),
    ))));
    assert!(
        format!("{robots_h:?}").contains("Terminal"),
        "Robots must become Terminal HandlerError, got {robots_h:?}"
    );

    // BrowserLane { retryable: false } (non-retryable) must become Terminal.
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

    // Timeout (retryable) must become Transient HandlerError.
    let timeout_h = job_error(collect_error(CrawlError::Fetch(FetchError::Timeout {
        url: "https://example.com".to_string(),
        timeout_secs: 30,
    })));
    assert!(
        format!("{timeout_h:?}").contains("Transient"),
        "Timeout must become Transient HandlerError, got {timeout_h:?}"
    );

    // Schema mismatch (non-fetch, terminal) must become Terminal.
    let schema_h = job_error(collect_error(CrawlError::Schema {
        url: "https://example.com".to_string(),
        detail: "missing required field".to_string(),
    }));
    assert!(
        format!("{schema_h:?}").contains("Terminal"),
        "Schema mismatch must become Terminal HandlerError, got {schema_h:?}"
    );
}
