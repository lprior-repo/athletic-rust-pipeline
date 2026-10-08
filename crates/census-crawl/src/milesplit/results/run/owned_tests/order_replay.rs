use super::super::replay::{entities, physical};
use super::*;

const APPLICATION_PHASE: &str = super::super::super::super::APPLICATION_PHASE;

fn journal_state(store: &Store) -> TestResult<serde_json::Value> {
    let phases = [
        APPLICATION_PHASE,
        super::super::super::super::RESULT_SET_PHASE,
        crate::milesplit::OWNED_CAPTURE_PHASE,
        crate::milesplit::OWNED_MEET_PHASE,
    ];
    Ok(serde_json::Value::Array(
        phases
            .into_iter()
            .map(|phase| {
                let keys = store.journal_keys(phase)?;
                let entries: std::collections::BTreeMap<_, _> = keys
                    .into_iter()
                    .map(|key| {
                        let payload = store.journal_payload(phase, &key)?;
                        Ok((key, payload))
                    })
                    .collect::<TestResult<_>>()?;
                Ok(json!({"phase": phase, "entries": entries}))
            })
            .collect::<TestResult<_>>()?,
    ))
}

#[derive(Debug, PartialEq)]
struct ProjectionState {
    entities: serde_json::Value,
    physical: Vec<(Table, census_store::TableWalk)>,
    journals: serde_json::Value,
    sequence: u64,
    physical_digest: String,
}

fn state(store: &Store) -> TestResult<ProjectionState> {
    let physical = physical(store)?;
    let tables: Vec<_> = physical.iter().map(|(table, _)| *table).collect();
    let snapshot = store.snapshot();
    Ok(ProjectionState {
        entities: entities(store)?,
        physical,
        journals: journal_state(store)?,
        sequence: snapshot.sequence(),
        physical_digest: snapshot.tables_digest(&tables)?,
    })
}

fn partition(
    fetcher: &Fetcher,
    first: &ResultSetRef,
) -> TestResult<super::super::super::super::ResultSetOptions> {
    let second = ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266815/raw")
        .ok_or("second source set")?;
    let mut controlled: serde_json::Value = serde_json::from_slice(TROY)?;
    controlled["data"][1]["meetResultsId"] = json!("1266815");
    seed_owned(fetcher, first, &serde_json::to_vec(&controlled)?)?;
    seed_metadata(fetcher, first)?;
    seed_metadata(fetcher, &second)?;
    let mut both = options(first);
    both.urls.extend(options(&second).urls);
    Ok(both)
}

async fn assert_repeat_is_immutable(
    store: &Store,
    fetcher: &Fetcher,
    request: &super::super::super::super::ResultSetOptions,
    expected: &ProjectionState,
    errors: u64,
) -> TestResult {
    let report = crate::milesplit::collect_result_sets(&context(store, fetcher)?, request).await?;
    check!(eq;
        (report.rows, report.errors, report.requests),
        (3, errors, 0)
    );
    check!(eq; state(store)?, *expected);
    Ok(())
}

#[test]
fn identical_completed_result_sets_preserve_all_physical_effects_when_url_order_reverses(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, first) = setup()?;
            let mut both = partition(&fetcher, &first)?;
            let historical = json!({"table": "meets.jsonl", "content_digest": "historical-effect"});
            store.journal_done(
                APPLICATION_PHASE,
                "meets.jsonl/historical-effect",
                &historical,
            )?;
            let initial =
                crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &both).await?;
            check!(eq; (initial.rows, initial.errors, initial.requests), (3, 0, 0));
            let before = state(&store)?;
            let selected: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq;
                selected.iter().map(|row| row.source_key.as_str()).collect::<std::collections::BTreeSet<_>>(),
                std::collections::BTreeSet::from([
                    "milesplit_result:201782263", "milesplit_result:201782277", "milesplit_result:201782806",
                ])
            );
            check!(eq; selected.iter().map(|row| &row.meet).collect::<std::collections::BTreeSet<_>>().len(), 1);
            check!(eq; selected.iter().map(|row| &row.team).collect::<std::collections::BTreeSet<_>>().len(), 2);
            check!(eq; selected.iter().map(|row| &row.athlete).collect::<std::collections::BTreeSet<_>>().len(), 2);
            check!(eq; selected.iter().filter_map(|row| row.source_athlete.as_ref().map(|owner| owner.id.as_str())).collect::<std::collections::BTreeSet<_>>(),
                std::collections::BTreeSet::from(["14222592", "11357806"]));
            for (key, mark) in [
                ("milesplit_result:201782263", census_domain::model::Mark::FieldImperial {
                    feet_mark: "13-9".into(), metres: census_domain::model::CentiMetres::new(419),
                }),
                ("milesplit_result:201782277", census_domain::model::Mark::FieldImperial {
                    feet_mark: "30-7".into(), metres: census_domain::model::CentiMetres::new(932),
                }),
                ("milesplit_result:201782806", census_domain::model::Mark::TimeSeconds(
                    census_domain::model::ExactSeconds::parse("12.40")?,
                )),
            ] {
                let row = selected.iter().find(|row| row.source_key == key).ok_or("selected source result")?;
                check!(eq; row.mark, mark);
                check!(row.evidence.iter().any(|evidence| evidence.note.as_deref()
                    .is_some_and(|note| note.contains("raw_metadata_capture") && note.contains("owned_capture"))));
            }
            assert_repeat_is_immutable(&store, &fetcher, &both, &before, 0).await?;
            both.urls.reverse();
            assert_repeat_is_immutable(&store, &fetcher, &both, &before, 0).await?;
            let recording = crate::recording::Recording::new();
            let ctx = AdapterContext {
                recording: Some(&recording),
                ..context(&store, &fetcher)?
            };
            let replay = crate::milesplit::collect_result_sets(&ctx, &both).await?;
            check!(eq; (replay.rows, replay.errors, replay.requests), (3, 0, 0));
            check!(
                recording.drain().is_empty(),
                "order-only replay stages no effects or witnesses"
            );
            check!(eq; state(&store)?, before);
            drop(store);
            let reopened = Store::open(_dir.path().join("store"))?;
            assert_repeat_is_immutable(&reopened, &fetcher, &both, &before, 0).await?;
            Ok(())
        })
}

#[test]
fn reversed_mixed_completed_and_unresolved_sets_preserve_retained_rows_and_valid_marks(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, first) = setup()?;
            let mut both = partition(&fetcher, &first)?;
            let mut source: serde_json::Value = serde_json::from_slice(TROY)?;
            source["data"][1]["meetResultsId"] = json!("1266815");
            source["data"][0]["gradYear"] = serde_json::Value::Null;
            seed_owned(&fetcher, &first, &serde_json::to_vec(&source)?)?;
            let report =
                crate::milesplit::collect_result_sets(&context(&store, &fetcher)?, &both).await?;
            check!(eq; (report.rows, report.errors, report.requests), (3, 1, 0));
            let marks: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            check!(eq;
                marks
                    .iter()
                    .map(|mark| mark.source_key.as_str())
                    .collect::<std::collections::BTreeSet<_>>(),
                std::collections::BTreeSet::from([
                    "milesplit_result:201782277",
                    "milesplit_result:201782806",
                ])
            );
            let receipts = store.journal_payloads(super::super::super::super::RESULT_SET_PHASE)?;
            check!(receipts
                .iter()
                .any(|row| row["disposition"] == "partial" && row["rsid"] == "1266814"));
            check!(receipts
                .iter()
                .any(|row| row["disposition"] == "projection_applied"
                    && row["rsid"] == "1266815"
                    && row["projected_rows"] == 1));
            let retained = receipts
                .iter()
                .find(|row| row["result_id"] == 201782263)
                .ok_or("original unresolved source row")?;
            check!(eq; retained["disposition"], "retained_unresolved");
            check!(eq; retained["cohort"], "missing");
            let before = state(&store)?;
            assert_repeat_is_immutable(&store, &fetcher, &both, &before, 1).await?;
            both.urls.reverse();
            assert_repeat_is_immutable(&store, &fetcher, &both, &before, 1).await?;
            Ok(())
        })
}
