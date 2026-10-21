use super::*;
use crate::restate_services::jurisdiction::team_collection;

pub(super) struct Fixture {
    pub(super) input: TeamsSourceRequest,
    pub(super) state: JurisdictionState,
    pub(super) request: TeamsReconciliationRequest,
    pub(super) ihsa: TeamsSourceOutcome,
    pub(super) continuation: TeamsContinuationOutcome,
}

pub(super) fn seed(store: &Store, failed: bool) -> TestResult<Fixture> {
    let mut input = request()?;
    input.source = "ihsa".into();
    store.bind_run(&RunManifest {
        store_identity: store.store_identity()?.as_str().into(),
        run: CensusRun::new(input.jurisdiction.season, input.jurisdiction.revision.get())
            .ok_or("invalid fixture run")?,
        cohort: GradYear::CO2027,
        jurisdictions: vec![input.jurisdiction.jurisdiction],
    })?;
    let transient = ledger::Attempt::Incomplete {
        outcome: stage(3, false, "ihsa first rows"),
        error: JobError::Transient {
            message: "IHSA retained timeout".into(),
        },
    };
    check!(matches!(
        attempt(store, &input, transient)?,
        ledger::Completion::Retry(_)
    ));
    let final_attempt = if failed {
        ledger::Attempt::Incomplete {
            outcome: stage(5, false, "ihsa retained rows"),
            error: JobError::Terminal {
                message: "IHSA public source refused".into(),
            },
        }
    } else {
        ledger::Attempt::Completed(stage(7, true, "ihsa settled rows"))
    };
    let ledger::Completion::Settled(ihsa) = attempt(store, &input, final_attempt)? else {
        return Err("IHSA fixture did not settle".into());
    };
    let progress = match &ihsa {
        TeamsSourceOutcome::Completed { progress, .. }
        | TeamsSourceOutcome::Terminal { progress, .. } => progress,
        _ => return Err("IHSA fixture lost its known history".into()),
    };
    check!(eq; progress.len(), 2);
    input.source = "milesplit".into();
    let holder = serde_json::json!({"origin":"https://oh.milesplit.com", "pid":12345});
    let message = format!("origin https://oh.milesplit.com is held by another census-service process (holder: {holder})");
    let ledger::Completion::Settled(original) = attempt(
        store,
        &input,
        ledger::Attempt::Failed(JobError::Terminal { message }),
    )?
    else {
        return Err("MileSplit fixture did not settle".into());
    };
    let original_outcome_digest = digest(&original)?;
    let authorization = ledger::origin_continuation::authorize(
        store,
        &key(&input),
        &input,
        &original_outcome_digest,
    )?;
    let ledger::AdmissionReplay::Reserved { attempt, .. } =
        ledger::origin_continuation::begin_at(store, &key(&input), &input, &authorization, 1)?
    else {
        return Err("continuation fixture did not reserve slot two".into());
    };
    check!(eq; attempt, 2);
    let ledger::Completion::Settled(outcome) = ledger::origin_continuation::finish(
        store,
        &key(&input),
        attempt,
        ledger::Attempt::Completed(stage(2, true, "continued school index")),
        &[],
    )?
    else {
        return Err("continuation fixture did not settle".into());
    };
    let mut aggregate =
        team_collection::empty_outcome("2026-10-02", CollectionDisposition::Complete);
    let mut failures = Vec::new();
    for (source, outcome) in [("ihsa", ihsa.clone()), ("milesplit", original)] {
        team_collection::retain(source, outcome, &mut aggregate, &mut failures)?;
    }
    let state = JurisdictionState {
        identity: WorkflowIdentity::jurisdiction(
            input.jurisdiction.jurisdiction,
            input.jurisdiction.season,
            input.jurisdiction.revision,
        )
        .as_str()
        .into(),
        history_window: Some(input.jurisdiction.history),
        plan: Some(SourcePlan {
            sweepable: vec!["ihsa".into(), "milesplit".into()],
            fingerprint: "retained-plan".into(),
            ..SourcePlan::default()
        }),
        teams: team_collection::finish(aggregate, failures, "2026-10-02"),
        ..JurisdictionState::default()
    };
    let request = TeamsReconciliationRequest {
        jurisdiction: input.jurisdiction.clone(),
        expected_teams_digest: digest(&state.teams)?,
        expected_plan_digest: digest(&state.plan)?,
    };
    Ok(Fixture {
        input,
        state,
        request,
        ihsa,
        continuation: TeamsContinuationOutcome {
            original_outcome_digest,
            outcome,
        },
    })
}

fn attempt(
    store: &Store,
    input: &TeamsSourceRequest,
    result: ledger::Attempt,
) -> TestResult<ledger::Completion> {
    let operation = key(input);
    let identity = ledger::register(store, &operation, input)?;
    let ledger::AdmissionReplay::Reserved { attempt, .. } = ledger::begin_at(
        store,
        &operation,
        input,
        &identity,
        ledger::baseline(store, &operation)?,
    )?
    else {
        return Err("fixture original attempt was not reserved".into());
    };
    Ok(ledger::finish(store, &operation, attempt, result, &[])?)
}

fn stage(records: usize, complete: bool, note: &str) -> StageOutcome {
    StageOutcome {
        records,
        at: "2026-10-02".into(),
        errors: Vec::new(),
        notes: vec![note.into()],
        disposition: if complete {
            CollectionDisposition::Complete
        } else {
            CollectionDisposition::Partial
        },
        unfinished: Vec::new(),
    }
}

impl Fixture {
    pub(super) fn configure(&mut self, mode: &str) -> TestResult {
        let plan = self.state.plan.as_mut().ok_or("fixture plan absent")?;
        match mode {
            "unselected" => plan.sweepable.retain(|source| source != "milesplit"),
            "unsettled" => plan.sweepable.insert(0, "mshsl".into()),
            "stale" | "settled" => {}
            _ => return Err("unknown fixture mode".into()),
        }
        self.request.expected_plan_digest = digest(&self.state.plan)?;
        if mode == "stale" {
            self.request.expected_teams_digest = "stale-parent".into();
        }
        Ok(())
    }

    pub(super) fn response(&self, call: &Frame) -> TestResult<Vec<u8>> {
        check!(eq; call.text(1)?, "TeamsSource");
        check!(codec::field(&call.payload, 3)?.is_none_or(<[u8]>::is_empty));
        let operation = call.text(5)?;
        for source in ["ihsa", "milesplit", "mshsl"] {
            let mut input = self.input.clone();
            input.source = source.into();
            if operation != key(&input) {
                continue;
            }
            if source == "milesplit" {
                check!(eq; call.text(2)?, "current");
                return Ok(serde_json::to_vec(&Some(&self.continuation))?);
            }
            check!(eq; call.text(2)?, "state");
            return Ok(serde_json::to_vec(&if source == "ihsa" {
                Some(&self.ihsa)
            } else {
                None
            })?);
        }
        Err(format!("unexpected child source {operation}").into())
    }
}

pub(super) fn snapshot(store: &Store) -> TestResult<serde_json::Value> {
    let mut phases = Vec::new();
    for phase in [ledger::PHASE, ledger::origin_continuation::PHASE] {
        let mut keys: Vec<_> = store.journal_keys(phase)?.into_iter().collect();
        keys.sort();
        let mut entries = Vec::new();
        for key in keys {
            entries.push((key.clone(), store.journal_payload(phase, &key)?));
        }
        phases.push((phase, entries));
    }
    Ok(
        serde_json::json!({"evidence":store.evidence_generation(), "derived":store.derived_generation(), "journals":phases}),
    )
}
