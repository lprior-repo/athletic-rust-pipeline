use super::*;
use crate::Application;

fn prepare_work(record: &Record, meter: &mut Meter) -> TestResult {
    meter.charge("replay preparation Store identity", 1)?;
    ledger::load(&[record], meter)?;
    prepare_tail(record, meter)
}

fn prepare_tail(record: &Record, meter: &mut Meter) -> TestResult {
    domain::command(record, meter)?;
    meter.charge("replay caller command proof issuance", 1)?;
    meter.charge("replay approval command origin", 1)?;
    meter.singles("replay approval measure and encode", 2)?;
    meter.charge("replay operation lookup", 1)?;
    meter.charge("replay full caller command equality", 1)?;
    meter.charge("replay approval digest copy", 1)
}

fn apply_work(record: &Record, meter: &mut Meter) -> TestResult {
    meter.singles("replay explicit approval and Store identity", 2)?;
    ledger::refresh(&[record], meter)?;
    meter.charge("replay refreshed operation lookup", 1)?;
    meter.charge("replay exact owner command and digest equality", 1)
}

fn exercise(target: Option<usize>) -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("replay-owner-phase-boundary")?;
    let inspection = fixture.inspect()?;
    let command = context.command("replay-owner-phase-boundary", &inspection)?;
    let modeled = Record::new(command.clone(), &inspection, &[], Some(&context))?;
    let receipt = written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command.clone(), &context)?,
    )?)?;
    let before = retained_state(&fixture.store)?;
    let mut meter = Meter::default();
    prepare_work(&modeled, &mut meter)?;
    let mut prepared = meter
        .result(
            fixture
                .store
                .prepare_evidence_subject_attachment_v1(command, Preparation::ReplayOnly),
            "replay-owner",
            "preparation",
        )?
        .ok_or("replay preparation unexpectedly refused")?;
    check!(prepared.is_replay());
    check!(eq; prepared.grant.usage().traversal_steps(), meter.admitted);
    if let Some(target) = target {
        let mut forecast = meter.clone();
        apply_work(&modeled, &mut forecast)?;
        check!(forecast.refusal.is_none());
        let padding = target
            .checked_sub(forecast.total)
            .ok_or("replay target below full work")?;
        meter.charge("replay controlled work boundary precharge", padding)?;
        prepared.grant.charge_steps(padding)?;
    }
    apply_work(&modeled, &mut meter)?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    let application = meter.result(
        fixture
            .store
            .apply_evidence_subject_attachment_v1(prepared, &approval, &digest),
        "replay-owner",
        "apply",
    )?;
    if target == Some(8193) {
        check!(application.is_none());
        let refusal = meter.refusal.as_ref().ok_or("replay work refusal absent")?;
        check!(eq; refusal.label, "replay exact owner command and digest equality");
        check!(eq; refusal.needed, 8193);
    } else {
        check!(eq; application, Some(Application::Repeated(receipt)));
    }
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn unchanged_replay_owner_matches_independent_full_load_and_fresh_refresh_work() -> TestResult {
    exercise(None)
}

#[test]
fn unchanged_replay_owner_completes_at_exact_same_grant_work_limit() -> TestResult {
    exercise(Some(8192))
}

#[test]
fn unchanged_replay_owner_one_beyond_work_limit_refuses_at_final_exact_equality() -> TestResult {
    exercise(Some(8193))
}

#[test]
fn exact_replay_preparation_work_exhaustion_occurs_before_digest_copy_issuance() -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("replay-owner-preparation-work")?;
    let inspection = fixture.inspect()?;
    let command = context.command("replay-owner-preparation-work", &inspection)?;
    let modeled = Record::new(command.clone(), &inspection, &[], Some(&context))?;
    written(apply(
        &fixture.store,
        prepare(&fixture.store, inspection, command.clone(), &context)?,
    )?)?;
    let mut inspection = fixture.inspect()?;
    let mut forecast = Meter::default();
    forecast.charge("replay preparation Store identity", 1)?;
    ledger::refresh(&[&modeled], &mut forecast)?;
    prepare_tail(&modeled, &mut forecast)?;
    let grant = inspection.grant.as_mut().ok_or("inspection grant absent")?;
    let padding = 8193usize
        .checked_sub(forecast.total)
        .and_then(|remaining| remaining.checked_sub(grant.usage().traversal_steps()))
        .ok_or("preparation work boundary padding underflow")?;
    grant.charge_steps(padding)?;
    let mut meter = Meter::default();
    meter.charge(
        "held inspection and controlled precharge",
        grant.usage().traversal_steps(),
    )?;
    meter.charge("replay preparation Store identity", 1)?;
    ledger::refresh(&[&modeled], &mut meter)?;
    prepare_tail(&modeled, &mut meter)?;
    let refusal = meter
        .refusal
        .as_ref()
        .ok_or("preparation work refusal absent")?;
    check!(eq; refusal.label, "replay approval digest copy");
    check!(eq; refusal.needed, 8193);
    let before = retained_state(&fixture.store)?;
    check!(meter
        .result(
            fixture.store.prepare_evidence_subject_attachment_v1(
                command,
                Preparation::New {
                    inspection,
                    context: &[]
                },
            ),
            "replay-owner",
            "preparation-copy-boundary"
        )?
        .is_none());
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}
