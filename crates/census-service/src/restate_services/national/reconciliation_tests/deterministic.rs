use super::super::publication_tests::codec;
use super::{
    fixtures, graph, protocol, retained_case, Journal, ReviewCase, Store, Table, TestResult,
};
use std::sync::Arc;

#[tokio::test]
async fn committed_rules_replay_after_their_run_completion_is_lost_without_graph_writes(
) -> TestResult {
    let controlled = protocol::controlled()?;
    let store = Arc::new(controlled.store().clone());
    let rows = fixtures::seed(&store)?;
    let old = retained_case("standing-during-lost-completion");
    store.replace(Table::ReviewCases, &old)?;
    let before = graph::read(&store)?;
    protocol::exercise(
        protocol::journal(controlled)?,
        protocol::Boundary::LostCompletion,
    )
    .await?;
    graph::unchanged(&store, &before, &rows)?;
    check!(store.scan::<ReviewCase>(Table::ReviewCases)?.contains(&old));
    check!(store
        .journal_payload("identity_retained_supersession_v1", &old.id)?
        .is_none());
    graph::legacy_write_refused(&store)
}

#[tokio::test]
async fn completed_rules_prefix_replays_exact_summary_without_retroactive_identity_writes(
) -> TestResult {
    let controlled = protocol::controlled()?;
    let store = Arc::new(controlled.store().clone());
    let rows = fixtures::seed(&store)?;
    let old = retained_case("standing-before-completed-rules");
    store.replace(Table::ReviewCases, &old)?;
    let before = graph::read(&store)?;
    let journal =
        protocol::exercise(protocol::journal(controlled)?, protocol::Boundary::Rules).await?;
    let retained = rules_result(&journal)?.to_vec();
    let journal = protocol::exercise(journal, protocol::Boundary::RetainedRules).await?;
    check!(eq; rules_result(&journal)?, retained.as_slice());
    graph::unchanged(&store, &before, &rows)?;
    check!(store.scan::<ReviewCase>(Table::ReviewCases)?.contains(&old));
    check!(store
        .journal_payload("identity_retained_supersession_v1", &old.id)?
        .is_none());
    graph::legacy_write_refused(&store)
}

pub(super) fn run_names(journal: &Journal) -> TestResult<Vec<String>> {
    journal
        .entries
        .iter()
        .filter(|entry| entry.kind == 0x0411)
        .try_fold(Vec::new(), |mut names, entry| {
            if let Some(name) = codec::field(&entry.payload, 12)? {
                let name = std::str::from_utf8(name)?;
                if name.starts_with("identity:") {
                    names.push(name.to_owned());
                }
            }
            Ok(names)
        })
}

fn rules_command(journal: &Journal) -> TestResult<&codec::Frame> {
    journal
        .entries
        .iter()
        .find(|entry| {
            entry.kind == 0x0411
                && codec::field(&entry.payload, 12)
                    .is_ok_and(|name| name == Some(b"identity:rules".as_slice()))
        })
        .ok_or_else(|| "deterministic Run absent".into())
}

fn rules_result(journal: &Journal) -> TestResult<&[u8]> {
    let completion = rules_command(journal)?.number(11)?;
    let result = journal
        .entries
        .iter()
        .find(|entry| entry.kind == 0x8011 && entry.number(1).is_ok_and(|id| id == completion))
        .ok_or("deterministic Run completion absent")?;
    Ok(codec::field(result.bytes(5)?, 1)?.ok_or("deterministic result absent")?)
}

fn lose_completion(journal: &mut Journal) -> TestResult {
    let completion = rules_command(journal)?.number(11)?;
    let position = journal
        .entries
        .iter()
        .position(|entry| entry.kind == 0x8011 && entry.number(1).is_ok_and(|id| id == completion))
        .ok_or("deterministic completion absent")?;
    journal.entries.truncate(position);
    Ok(())
}

pub(super) async fn reach_rules(
    journal: &mut Journal,
    client: &reqwest::Client,
    url: &str,
) -> TestResult {
    for _ in 0..256 {
        journal.step(client, url).await?;
        if run_names(journal)?.contains(&"identity:rules".to_owned()) {
            return Ok(());
        }
    }
    Err("deterministic rules did not execute within the journal budget".into())
}

pub(super) async fn lost(
    store: &Store,
    journal: &mut Journal,
    client: &reqwest::Client,
    url: &str,
) -> TestResult {
    reach_rules(journal, client, url).await?;
    check!(eq; run_names(journal)?, vec!["identity:rules"]);
    let rows = graph::state(store)?;
    let before = graph::read(store)?;
    lose_completion(journal)?;
    let replayed = journal.step(client, url).await?;
    check!(replayed.iter().any(|entry| entry.kind == 0x0005));
    check!(!replayed.iter().any(|entry| entry.kind == 0x0411));
    check!(eq; graph::state(store)?, rows);
    graph::unchanged(store, &before, &[])?;
    check!(eq; run_names(journal)?, vec!["identity:rules"]);
    journal.drive(client, url).await?;
    check!(journal.held.is_some());
    check!(eq; run_names(journal)?.iter().filter(|name| name.as_str() == "identity:rules").count(), 1);
    Ok(())
}

pub(super) async fn retained(
    store: &Store,
    journal: &mut Journal,
    client: &reqwest::Client,
    url: &str,
) -> TestResult {
    let prefix = journal.entries.clone();
    let result = rules_result(journal)?.to_vec();
    let before = graph::read(store)?;
    let reply = journal.step(client, url).await?;
    check!(!reply.iter().any(|entry| {
        entry.kind == 0x0411
            && codec::field(&entry.payload, 12)
                .is_ok_and(|name| name == Some(b"identity:rules".as_slice()))
    }));
    check!(eq; journal.entries.get(..prefix.len()), Some(prefix.as_slice()));
    check!(eq; rules_result(journal)?, result.as_slice());
    journal.drive(client, url).await?;
    check!(journal.held.is_some());
    graph::unchanged(store, &before, &[])?;
    check!(eq; run_names(journal)?.iter().filter(|name| name.as_str() == "identity:rules").count(), 1);
    Ok(())
}
