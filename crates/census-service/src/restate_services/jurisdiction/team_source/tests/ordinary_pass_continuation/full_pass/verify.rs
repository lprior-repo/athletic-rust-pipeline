use super::*;

pub(super) async fn run(fixture: &Fixture, url: &str, mode: &str, tamper: bool) -> TestResult {
    let client = reqwest::Client::builder()
        .http2_prior_knowledge()
        .timeout(Duration::from_secs(5))
        .build()?;
    let mut journal = prefix::completed(fixture)?;
    if tamper {
        prefix::tamper(&mut journal, fixture)?;
        let frames =
            super::super::super::replay::invoke(&client, url, &fixture.state.identity, &journal)
                .await?;
        check!(frames
            .iter()
            .any(|frame| frame.kind == 0x0002
                && frame.text(2).is_ok_and(|message| !message.is_empty())));
        check!(!frames
            .iter()
            .any(|frame| matches!(frame.kind, 0x0401 | 0x0403 | 0x040D | 0x0411 | 0x0005)));
        return Ok(());
    }
    let old = journal.clone();
    let (output, saved, calls, mut boundary) =
        super::journal::drive(&client, url, fixture, &mut journal, mode).await?;
    check!(eq; journal.get(..old.len()).ok_or("retained prefix shortened")?, old.as_slice());
    check!(eq; calls.iter().map(|call| call.split(':').take(2).collect::<Vec<_>>().join(":")).collect::<Vec<_>>(),
        vec!["TeamsSource:current", "TeamsSource:state", "NationalCensus:request", "TeamsSource:state"]);
    let stored = super::journal::saved(&saved)?;
    verify_state(fixture, &stored, mode)?;
    let owed: JurisdictionOwed =
        serde_json::from_slice(field(output.bytes(14)?, 1)?.ok_or("owed output absent")?)?;
    check!(eq; owed.identity, fixture.state.identity);
    check!(eq; owed.stages_run, vec!["teams"]);
    check!(eq; owed.owed.iter().any(|stage| stage == "teams"), mode == "settled_failure");
    let (repeated, saves, new_calls, _) =
        super::journal::drive(&client, url, fixture, &mut boundary, mode).await?;
    check!(new_calls.is_empty());
    check!(eq; saves.len(), 1);
    check!(eq; repeated.payload, output.payload);
    let replay =
        super::super::super::replay::invoke(&client, url, &fixture.state.identity, &journal)
            .await?;
    check!(!replay
        .iter()
        .any(|frame| matches!(frame.kind, 0x0403 | 0x040D | 0x0411 | 0x0005)));
    check!(eq; replay.iter().find(|frame| frame.kind == 0x0401).ok_or("replay output absent")?.payload, output.payload);
    Ok(())
}

fn verify_state(fixture: &Fixture, state: &JurisdictionState, mode: &str) -> TestResult {
    let mut before = serde_json::to_value(&fixture.state)?;
    let mut after = serde_json::to_value(state)?;
    before
        .as_object_mut()
        .ok_or("before object absent")?
        .remove("teams");
    after
        .as_object_mut()
        .ok_or("after object absent")?
        .remove("teams");
    check!(eq; before, after);
    check!(state
        .rosters
        .as_ref()
        .is_some_and(crate::census::StateProgress::is_terminal));
    let outcome = if mode == "settled_success" {
        let TeamsStage::Completed(outcome) = &state.teams else {
            return Err("continued complete parent missing".into());
        };
        check!(eq; outcome.outcome().records, 9);
        check!(outcome.outcome().errors.is_empty());
        outcome.outcome()
    } else {
        let TeamsStage::Failed(TeamsFailure::SourceFailures {
            outcome, failures, ..
        }) = &state.teams
        else {
            return Err("other source failure was lost".into());
        };
        let [failure] = failures.as_slice() else {
            return Err("other source failure duplicated or replaced".into());
        };
        check!(eq; failure.source, "ihsa");
        let mut source = fixture.source.clone();
        source.source = "ihsa".into();
        let original = ledger::status(&fixture.store, &key(&source), Some(&source))?
            .ok_or("IHSA original absent")?;
        check!(eq; serialized_digest(&failure.outcome)?, serialized_digest(&original)?);
        check!(eq; outcome.records, 7);
        check!(eq; outcome.errors, vec!["ihsa: terminal: retained IHSA refusal"]);
        outcome
    };
    let first = if mode == "settled_success" {
        "ihsa: retained IHSA success"
    } else {
        "ihsa: retained IHSA terminal rows"
    };
    check!(eq; outcome.notes, vec![first, "milesplit: continued directory"]);
    check!(outcome.unfinished.is_empty());
    Ok(())
}
