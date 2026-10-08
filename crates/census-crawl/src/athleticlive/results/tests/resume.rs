use super::*;
use crate::net::cache::content_digest;
use serde_json::Value;
use std::collections::{BTreeSet, HashSet};

fn resume_options(paths: Vec<String>) -> ResultOptions {
    ResultOptions {
        documents: paths,
        ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
    }
}

fn receipts_of_path(store: &Store, path: &str) -> TestResult<Vec<(String, serde_json::Value)>> {
    Ok(receipts(store)?
        .into_iter()
        .filter(|(_, payload)| payload["path"].as_str() == Some(path))
        .collect())
}

#[test]
fn a_capture_whose_bytes_changed_is_read_again_instead_of_resumed() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher) = scratch()?;
            let doc = parse_event_document(&event_doc_url(2_150_205), XC_STATE)?;
            write_schools(
                &store,
                &labels_of(&labelled_schools(&doc, UsJurisdiction::Iowa)),
            )?;
            let path = stage_capture(&dir, "event-doc-2150205.json", XC_STATE)?;
            let options = resume_options(vec![path.clone()]);

            let first = collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; first.rows, 136);
            let before = content_digest(XC_STATE.as_bytes());

            let mut document: Value = serde_json::from_str(XC_STATE)?;
            let rows = document
                .pointer_mut("/_source/r")
                .and_then(Value::as_array_mut)
                .ok_or("the capture carries its rows")?;
            rows.truncate(1);
            let changed = document.to_string();
            std::fs::write(&path, &changed)?;
            let after = content_digest(changed.as_bytes());
            check!(ne; after, before, "the bytes under the path changed");

            let second = collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; second.rows, 1, "the changed bytes are read, not skipped");
            check!(eq; second.errors, 0, "{}", joined(&second));
            check!(
                !joined(&second).contains("already journaled"),
                "the path alone no longer proves the capture was read: {}",
                joined(&second)
            );

            let read = receipts_of_path(&store, &path)?;
            check!(eq; read.len(), 2, "one receipt per read of the path");
            let digests: BTreeSet<String> = read
                .iter()
                .filter_map(|(_, payload)| payload["digest"].as_str().map(str::to_string))
                .collect();
            check!(eq;
                digests,
                BTreeSet::from([before.clone(), after.clone()]),
                "each receipt binds the bytes it read"
            );
            let keys: BTreeSet<String> = read.iter().map(|(key, _)| key.clone()).collect();
            check!(eq; keys.len(), 2, "the receipt key carries the digest: {keys:?}");
            check!(eq;
                captures(&store)?,
                HashSet::from([before, after]),
                "both bodies stay archived"
            );
            Ok(())
        })
}

#[test]
fn bytes_journaled_for_one_meet_are_refused_for_another() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher) = scratch()?;
            let doc = parse_event_document(&event_doc_url(2_150_205), XC_STATE)?;
            write_schools(
                &store,
                &labels_of(&labelled_schools(&doc, UsJurisdiction::Iowa)),
            )?;
            let path = stage_capture(&dir, "state-final.json", XC_STATE)?;

            let iowa = resume_options(vec![path.clone()]);
            let first = collect(&context(&store, &fetcher)?, &iowa).await?;
            check!(eq; first.errors, 0, "{}", joined(&first));

            let michigan = ResultOptions {
                documents: vec![path.clone()],
                ..ResultOptions::for_meet(mits_meet(), OBSERVED_ON)
            };
            let second = collect(&context(&store, &fetcher)?, &michigan).await?;
            check!(eq; second.rows, 0, "the bytes are not projected under a second meet");
            check!(eq; second.errors, 1, "{}", joined(&second));
            check!(
                joined(&second).contains("refusing to project"),
                "{}",
                joined(&second)
            );
            check!(eq;
                receipts(&store)?.len(),
                1,
                "no receipt lands for the refused meet"
            );
            Ok(())
        })
}

#[test]
fn a_missing_capture_file_resumes_from_the_archived_body() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher) = scratch()?;
            let doc = parse_event_document(&event_doc_url(2_150_205), XC_STATE)?;
            write_schools(
                &store,
                &labels_of(&labelled_schools(&doc, UsJurisdiction::Iowa)),
            )?;
            let path = stage_capture(&dir, "event-doc-2150205.json", XC_STATE)?;
            let mut options = resume_options(vec![path.clone()]);
            options.capture_metadata = super::captures::documents(&options.documents)?;

            let first = collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; first.rows, 136);
            std::fs::remove_file(&path)?;

            let second = super::super::collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; second.rows, 0, "the archived bytes prove the capture was read");
            check!(eq; second.errors, 0, "{}", joined(&second));
            check!(
                joined(&second).contains("captures already journaled by an earlier run: 1"),
                "{}",
                joined(&second)
            );
            check!(eq;
                receipts(&store)?.len(),
                1,
                "the same bytes leave no second receipt"
            );
            Ok(())
        })
}

#[test]
fn identical_bytes_at_a_second_path_are_one_archived_body() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher) = scratch()?;
            let doc = parse_event_document(&event_doc_url(2_150_205), XC_STATE)?;
            write_schools(
                &store,
                &labels_of(&labelled_schools(&doc, UsJurisdiction::Iowa)),
            )?;
            let first_path = stage_capture(&dir, "state-final.json", XC_STATE)?;
            let copy_path = stage_capture(&dir, "state-final-copy.json", XC_STATE)?;
            let options = resume_options(vec![first_path.clone(), copy_path.clone()]);

            let report = collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; report.errors, 0, "{}", joined(&report));
            check!(eq;
                captures(&store)?.len(),
                1,
                "identical bytes are archived once"
            );
            check!(eq;
                receipts(&store)?.len(),
                2,
                "each path names its own receipt"
            );
            check!(eq;
                receipts_of_path(&store, &copy_path)?.len(),
                1,
                "the copy's receipt is the copy's own"
            );
            Ok(())
        })
}

#[test]
fn a_store_holding_retired_path_receipts_refuses_to_resume() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for retired in ["athleticlive_results_v2", "athleticlive_results_effect_v1"] {
                let (dir, store, fetcher) = scratch()?;
                let doc = parse_event_document(&event_doc_url(2_150_205), XC_STATE)?;
                write_schools(
                    &store,
                    &labels_of(&labelled_schools(&doc, UsJurisdiction::Iowa)),
                )?;
                let path = stage_capture(&dir, "event-doc-2150205.json", XC_STATE)?;
                let options = resume_options(vec![path.clone()]);

                let ctx = context(&store, &fetcher)?;
                let mut page = ctx.write_batch();
                page.journal_done(
                    retired,
                    &path,
                    &serde_json::json!({"role": "event", "events": 0}),
                )?;
                page.commit()?;

                let error = match collect(&context(&store, &fetcher)?, &options).await {
                    Err(error) => error,
                    Ok(report) => {
                        return Err(format!(
                            "a store holding the retired phase must not resume: {}",
                            joined(&report)
                        )
                        .into())
                    }
                };
                check!(error.to_string().contains("retired"), "{error}");
                check!(
                    receipts(&store)?.is_empty(),
                    "the refusal leaves the retired journal alone"
                );
            }
            Ok(())
        })
}
