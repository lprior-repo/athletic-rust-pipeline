use std::path::PathBuf;

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
