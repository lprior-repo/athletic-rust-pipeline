use std::path::PathBuf;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;

use crate::census::{Revision, WorkflowIdentity};

use super::*;

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
        season: SchoolYear(2026),
        revision: Revision(1),
        jurisdictions,
        refresh: false,
        limit_per_state: None,
        concurrency: 4,
        observed_on: None,
    }
}

#[test]
fn an_empty_jurisdiction_list_covers_every_state_in_declaration_order() {
    let targets = national::targets(&national_request(Vec::new())).unwrap();
    assert_eq!(targets.len(), UsJurisdiction::ALL.len());
    let jurisdictions: Vec<UsJurisdiction> = targets.iter().map(|row| row.0).collect();
    assert_eq!(jurisdictions, UsJurisdiction::ALL.to_vec());
    // Each state is addressed by the identity of its own census, not by the position it was pushed.
    for (jurisdiction, key) in &targets {
        assert_eq!(
            key.as_str(),
            WorkflowIdentity::jurisdiction(*jurisdiction, SchoolYear(2026), Revision(1)).as_str()
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
    assert_eq!(options.school_year, SchoolYear(2026));
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
