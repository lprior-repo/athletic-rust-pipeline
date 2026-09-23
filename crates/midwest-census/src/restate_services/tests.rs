use std::num::NonZeroUsize;
use std::path::PathBuf;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;

use crate::census::{
    owed_source_objects, MeetCensus, Revision, SourceObject, StateProgress, WorkflowIdentity,
};
use census_report::report::ReportError;
use census_crawl::registry::{
    AccessClass, SourceAdmission, SourceCapabilities, SourceDescriptor, TransportKind,
};
use census_store::StoreError;

use super::*;
use census_report::report::Scope;
use crate::restate_services::plan::classify_access;
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
    // `midwest-census report` without `--core` reports every source, so the service must too.
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

#[test]
fn rows_without_an_id_are_rejected_by_the_store_and_nothing_is_written() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(dir.path()).unwrap();
    let rows = vec![serde_json::json!({"name": "no id here"})];
    assert!(append_observations(&store, Table::Schools, &rows).is_err());
    assert_eq!(store.stats().unwrap().observations, 0);

    let good = vec![serde_json::json!({"id": "school:wi:test", "name": "Test"})];
    assert_eq!(
        append_observations(&store, Table::Schools, &good).unwrap(),
        1
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
    let refused = append_observations(&store, Table::Schools, &rows)
        .map_err(JobError::from)
        .expect_err("a batch over the ceiling is refused");
    assert!(matches!(refused, JobError::Terminal { .. }));
    assert_eq!(store.stats().unwrap().observations, 0);
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
        plan: SourcePlan::of(&plan(UsJurisdiction::Wisconsin, BrowserLaneState::Absent)),
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
