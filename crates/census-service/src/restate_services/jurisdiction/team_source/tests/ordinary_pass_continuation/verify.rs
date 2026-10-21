use super::*;
use crate::restate_services::wire::{TeamsFailure, TeamsStage};
use census_domain::model::serialized_digest;
use std::time::Duration;

pub(super) async fn exercise(mode: &str) -> TestResult {
    let fixture = fixture::Fixture::new(mode)?;
    let before = fixture.snapshot()?;
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let url = format!(
        "http://{}/invoke/OrdinaryPassSuffix/consume",
        listener.local_addr()?
    );
    let endpoint = Endpoint::builder()
        .bind(OrdinaryPassSuffix {
            owner: fixture.owner.clone(),
        })
        .build();
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel();
    let server = HttpServer::new(endpoint).serve_with_cancel(listener, async move {
        let _ = stop_rx.await;
    });
    let assertions = async {
        let result =
            tokio::time::timeout(Duration::from_secs(30), verify(&fixture, &url, mode)).await;
        stop_tx
            .send(())
            .map_err(|_| "suffix endpoint lost shutdown owner")?;
        result?
    };
    let ((), result) = tokio::join!(server, assertions);
    let drained = fixture.region.drain(Duration::from_secs(5)).await?;
    check!(eq; (drained.remaining, drained.panicked), (0, 0));
    result?;
    check!(eq; fixture.snapshot()?, before);
    check!(!fixture.store.journal_contains(
        ledger::origin_continuation::PHASE,
        &format!("{}/attempt/3/reserved", key(&fixture.source))
    )?);
    check!(!fixture.store.journal_contains(
        ledger::PHASE,
        &format!("{}/attempt/2/reserved", key(&fixture.source))
    )?);
    Ok(())
}

async fn verify(fixture: &fixture::Fixture, url: &str, mode: &str) -> TestResult {
    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .timeout(Duration::from_secs(5))
        .build()?;
    let initial = journal::initial(&fixture.state, &fixture.source.jurisdiction)?;
    let mut replay = initial.clone();
    let (output, saved, calls, mut boundary) =
        journal::drive(&client, url, fixture, &mut replay, mode).await?;
    if mode.starts_with("inventory_")
        || matches!(
            mode,
            "foreign_bound" | "foreign_plan" | "wrong_digest" | "unrelated_parent"
        )
    {
        check!(saved.is_empty());
        if mode.starts_with("inventory_") {
            check!(calls.is_empty());
            check!(eq; replay, initial);
            let (again, saves, calls, _) =
                journal::drive(&client, url, fixture, &mut replay, mode).await?;
            check!(saves.is_empty() && calls.is_empty());
            check!(eq; replay, initial);
            check!(eq; again.payload, output.payload);
        }
        let message = codec::field(output.bytes(15)?, 2)?.ok_or("failure message absent")?;
        let expected = match mode {
            value if value.starts_with("inventory_") => "retained source plan is ambiguous",
            "foreign_bound" => "jurisdiction differs from full bound national request",
            "foreign_plan" => "public resource service binding",
            "wrong_digest" => "continuation original outcome digest mismatch",
            "unrelated_parent" => "parent differs from original and settled continuation",
            _ => return Err("unexpected refusal mode".into()),
        };
        check!(std::str::from_utf8(message)?.contains(expected));
        return Ok(());
    }
    let returned = journal::output(&output)?;
    if matches!(mode, "pending" | "unsettled_other") {
        check!(saved.is_empty());
        check!(eq; serde_json::to_vec(&returned)?, serde_json::to_vec(&fixture.state)?);
        check!(eq; calls.len(), if mode == "pending" { 1 } else { 4 });
        return Ok(());
    }
    let stored = journal::saved(&saved)?;
    check!(eq; serde_json::to_vec(&stored)?, serde_json::to_vec(&returned)?);
    same_non_teams(&fixture.state, &stored)?;
    retained_sources(fixture, &stored, mode)?;
    check!(eq; calls.iter().map(|call| call.split(':').take(2).collect::<Vec<_>>().join(":")).collect::<Vec<_>>(), vec!["TeamsSource:current", "TeamsSource:state", "NationalCensus:request", "TeamsSource:state"]);
    let (again, saved_again, new_calls, _) =
        journal::drive(&client, url, fixture, &mut boundary, mode).await?;
    check!(new_calls.is_empty());
    check!(eq; saved_again.len(), 1);
    check!(eq; again.payload, output.payload);
    let replayed =
        super::super::replay::invoke(&client, url, &fixture.state.identity, &replay).await?;
    check!(!replayed
        .iter()
        .any(|frame| matches!(frame.kind, 0x0403 | 0x040D | 0x0411 | 0x0005)));
    check!(eq; replayed.iter().find(|frame| frame.kind == 0x0401).ok_or("replay output absent")?.payload, output.payload);
    let mut repeated = journal::initial(&stored, &fixture.source.jurisdiction)?;
    let (again, saves, calls, _) =
        journal::drive(&client, url, fixture, &mut repeated, mode).await?;
    check!(saves.is_empty());
    check!(eq; serde_json::to_vec(&journal::output(&again)?)?, serde_json::to_vec(&stored)?);
    check!(eq; calls.len(), if mode == "continued_failure" { 2 } else { 0 });
    Ok(())
}

fn same_non_teams(before: &JurisdictionState, after: &JurisdictionState) -> TestResult {
    let mut before = serde_json::to_value(before)?;
    let mut after = serde_json::to_value(after)?;
    before
        .as_object_mut()
        .ok_or("before is not an object")?
        .remove("teams");
    after
        .as_object_mut()
        .ok_or("after is not an object")?
        .remove("teams");
    check!(eq; before, after);
    Ok(())
}

fn retained_sources(
    fixture: &fixture::Fixture,
    state: &JurisdictionState,
    mode: &str,
) -> TestResult {
    let current = fixture.current.as_ref().ok_or("current absent")?;
    let outcome = match &state.teams {
        TeamsStage::Completed(outcome) => outcome.outcome(),
        TeamsStage::Failed(TeamsFailure::SourceFailures {
            outcome, failures, ..
        }) => {
            let expected = if mode == "continued_failure" { 2 } else { 1 };
            check!(eq; failures.len(), expected);
            let first = failures.first().ok_or("IHSA failure absent")?;
            check!(eq; first.source, "ihsa");
            let mut input = fixture.source.clone();
            input.source = "ihsa".into();
            let original = ledger::status(&fixture.store, &key(&input), Some(&input))?
                .ok_or("IHSA original absent")?;
            check!(eq; serialized_digest(&first.outcome)?, serialized_digest(&original)?);
            if mode == "continued_failure" {
                let last = failures.last().ok_or("MS continuation failure absent")?;
                check!(eq; last.source, "milesplit");
                check!(eq; serialized_digest(&last.outcome)?, serialized_digest(&current.outcome)?);
            }
            outcome
        }
        _ => return Err("unexpected parent teams stage".into()),
    };
    if mode == "settled_success" {
        check!(matches!(state.teams, TeamsStage::Completed(_)));
        check!(eq; outcome.records, 9);
        check!(eq; outcome.notes, vec!["ihsa: retained IHSA success", "milesplit: continued directory"]);
        check!(outcome.errors.is_empty());
    } else if mode == "continued_failure" {
        check!(eq; outcome.records, 5);
        check!(eq; outcome.errors, vec!["ihsa: terminal: retained IHSA refusal", "milesplit: terminal: continued source refused"]);
    } else {
        check!(eq; outcome.records, 7);
        check!(eq; outcome.notes, vec!["ihsa: retained IHSA terminal rows", "milesplit: continued directory"]);
        check!(eq; outcome.errors, vec!["ihsa: terminal: retained IHSA refusal"]);
    }
    check!(outcome.unfinished.is_empty());
    Ok(())
}
