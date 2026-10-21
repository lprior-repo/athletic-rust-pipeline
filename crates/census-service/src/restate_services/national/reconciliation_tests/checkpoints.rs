use super::super::publication_tests::codec;
use super::{
    fixtures, graph, protocol, retained_case, ReviewCase, ReviewVerdictRecord, Table, TestResult,
};
use std::sync::Arc;

#[tokio::test]
async fn national_visits_all_review_checkpoints_and_replays_without_new_jobs() -> TestResult {
    let controlled = protocol::controlled()?;
    let store = Arc::new(controlled.store().clone());
    let rows = fixtures::seed(&store)?;
    let cases: Vec<_> = (0..258)
        .map(|index| retained_case(&format!("unrelated:{index:03}")))
        .collect();
    store.replace_many(Table::ReviewCases, &cases)?;
    let before = graph::read(&store)?;
    let journal =
        protocol::exercise(protocol::journal(controlled)?, protocol::Boundary::Workbook).await?;
    check!(eq; super::deterministic::run_names(&journal)?, vec!["identity:rules", "identity:advice:0", "identity:advice:1", "identity:advice:2", "identity:reconcile"]);
    let mut cursors = Vec::new();
    for entry in journal.entries.iter().filter(|entry| entry.kind == 0x0403) {
        if entry.text(1)? == "identity:advice" {
            let value = codec::field(entry.bytes(3)?, 1)?.ok_or("advice state value missing")?;
            let (_, cursor): (String, Option<String>) = serde_json::from_slice(value)?;
            cursors.push(cursor);
        }
    }
    check!(matches!(cursors.as_slice(), [Some(first), Some(second), None] if second > first));
    let verdicts = store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
    let retained = store.scan::<ReviewCase>(Table::ReviewCases)?;
    for case in &cases {
        check!(retained.contains(case));
        check!(verdicts
            .iter()
            .any(|verdict| verdict.case_id == case.id && !verdict.accepted));
    }
    graph::unchanged(&store, &before, &rows)?;
    graph::legacy_write_refused(&store)
}
