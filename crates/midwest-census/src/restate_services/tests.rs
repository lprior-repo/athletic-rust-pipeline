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
    assert!(append_observations(&store, Table::Schools, &rows).is_err());
    assert_eq!(store.stats().unwrap().observations, 0);
}

#[test]
fn transient_jobs_are_retryable_and_panics_are_terminal() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let transient = runtime.block_on(blocking(|| -> anyhow::Result<u8> {
        anyhow::bail!("disk went away")
    }));
    assert!(matches!(transient, Err(JobError::Transient { .. })));
    let panicked = runtime.block_on(blocking(|| -> anyhow::Result<u8> {
        panic!("boom");
    }));
    assert!(matches!(panicked, Err(JobError::Terminal { .. })));
}
