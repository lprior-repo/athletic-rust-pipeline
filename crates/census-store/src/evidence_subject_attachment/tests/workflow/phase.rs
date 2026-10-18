use super::fixture::*;
use crate::evidence_subject_attachment::{
    AttachmentInspectionRequestV1, EvidenceSubjectAttachmentPreparationV1 as Preparation,
};
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::Store;
use census_domain::evidence_subject_attachment::*;
use meter::Meter;
use model::Record;

mod domain;
mod ledger;
mod meter;
mod model;
mod operation;

struct PhaseCase<'a> {
    operation: &'static str,
    request: AttachmentInspectionRequestV1,
    original: Option<&'a ReviewedReferenceRecordV1>,
    supplied: Option<&'a Context>,
    final_step_target: Option<usize>,
}

fn exercise(
    store: &Store,
    prior: &mut Vec<Record>,
    case: PhaseCase<'_>,
) -> TestResult<Option<ReviewedReferenceRecordV1>> {
    let before = retained_state(store)?;
    let references = prior.iter().collect::<Vec<_>>();
    let mut meter = Meter::default();
    operation::inspect(&references, &case.request, &mut meter)?;
    let Some(inspection) = meter.result(
        store.inspect_evidence_subject_attachment_v1(case.request),
        case.operation,
        "inspection",
    )?
    else {
        check!(eq; retained_state(store)?, before);
        return Ok(None);
    };
    check!(eq; inspection.grant.as_ref().ok_or("inspection grant absent")?.usage().traversal_steps(), meter.admitted);
    let command = match (case.original, case.supplied) {
        (Some(original), None) => next_command(original, case.operation, &inspection)?,
        (None, Some(context)) => context.command(case.operation, &inspection)?,
        _ => return Err("phase fixture needs original or new context".into()),
    };
    let candidate = Record::new(command.clone(), &inspection, prior, case.supplied)?;
    operation::prepare(&references, &candidate, &mut meter)?;
    let supplied = match case.supplied {
        Some(context) => context.borrowed(),
        None => Vec::new(),
    };
    let prepared = store.prepare_evidence_subject_attachment_v1(
        command,
        Preparation::New {
            inspection,
            context: &supplied,
        },
    );
    let Some(mut prepared) = meter.result(prepared, case.operation, "preparation")? else {
        check!(eq; retained_state(store)?, before);
        return Ok(None);
    };
    check!(eq; prepared.grant.usage().traversal_steps(), meter.admitted);
    if let Some(target) = case.final_step_target {
        let mut forecast = meter.clone();
        operation::apply(&references, &candidate, &mut forecast)?;
        check!(forecast.refusal.is_none());
        let padding = target
            .checked_sub(forecast.total)
            .ok_or("boundary target below complete work")?;
        meter.charge("controlled boundary precharge", padding)?;
        prepared.grant.charge_steps(padding)?;
    }
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    operation::apply(&references, &candidate, &mut meter)?;
    let Some(application) = meter.result(
        store.apply_evidence_subject_attachment_v1(prepared, &approval, &digest),
        case.operation,
        "apply",
    )?
    else {
        check!(eq; retained_state(store)?, before);
        return Ok(None);
    };
    written(application)?;
    prior.push(candidate);
    complete_history(store, prior)
}

fn complete_history(
    store: &Store,
    prior: &[Record],
) -> TestResult<Option<ReviewedReferenceRecordV1>> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let read = snapshot.read_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(eq; read.records().len(), prior.len());
    for (actual, expected) in read.records().iter().zip(prior) {
        check!(eq; actual.command, expected.command);
    }
    for (name, rows) in retained_state(store)? {
        if name != crate::JOURNAL {
            continue;
        }
        for (key, value) in rows {
            if key.starts_with(b"evidence_subject\0") {
                check!(value.len() <= crate::MAX_JOURNAL_VALUE_BYTES);
                check!(key.starts_with(b"evidence_subject\0attachment/v1/record/"));
                check!(eq; key.len(), b"evidence_subject\0attachment/v1/record/".len().checked_add(20).ok_or("key size overflow")?);
            }
        }
    }
    Ok(Some(
        read.records()
            .last()
            .ok_or("completed record absent")?
            .clone(),
    ))
}

#[test]
fn controlled_attach_revoke_and_new_target_first_match_independent_phase_work_or_exact_admission_refusal(
) -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("phase-cost-chain")?;
    let mut prior = Vec::new();
    let first = exercise(
        &fixture.store,
        &mut prior,
        PhaseCase {
            operation: "phase-first",
            request: AttachmentInspectionRequestV1::Attach {
                target: fixture.first.public_id().clone(),
                prior: AttachmentPredecessorV1::First,
            },
            original: None,
            supplied: Some(&context),
            final_step_target: None,
        },
    )?;
    let Some(first) = first else {
        return Ok(());
    };
    let second_target = fixture.second_birth()?;
    let revoked = exercise(
        &fixture.store,
        &mut prior,
        PhaseCase {
            operation: "phase-revoke",
            request: AttachmentInspectionRequestV1::Revoke {
                prior: first.reference(),
            },
            original: Some(&first),
            supplied: None,
            final_step_target: None,
        },
    )?;
    let Some(_revoked) = revoked else {
        return Ok(());
    };
    let retargeted = exercise(
        &fixture.store,
        &mut prior,
        PhaseCase {
            operation: "phase-new-first",
            request: AttachmentInspectionRequestV1::Attach {
                target: second_target.public_id().clone(),
                prior: AttachmentPredecessorV1::First,
            },
            original: Some(&first),
            supplied: None,
            final_step_target: None,
        },
    )?;
    if let Some(retargeted) = retargeted {
        check!(eq; prior.len(), 3);
        check!(eq; retargeted.command.target.public_id, *second_target.public_id());
    }
    Ok(())
}

#[test]
fn first_apply_accepts_exact_independently_counted_original_grant_boundary() -> TestResult {
    boundary(AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS, true)
}

#[test]
fn first_apply_refuses_one_beyond_independently_counted_original_grant_boundary_without_writes(
) -> TestResult {
    boundary(
        AttachmentReplayGrantV1::MAX_TRAVERSAL_STEPS
            .checked_add(1)
            .ok_or("phase work limit overflow")?,
        false,
    )
}

fn boundary(target: usize, completes: bool) -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("phase-exact-boundary")?;
    let mut prior = Vec::new();
    let result = exercise(
        &fixture.store,
        &mut prior,
        PhaseCase {
            operation: "phase-boundary",
            request: AttachmentInspectionRequestV1::Attach {
                target: fixture.first.public_id().clone(),
                prior: AttachmentPredecessorV1::First,
            },
            original: None,
            supplied: Some(&context),
            final_step_target: Some(target),
        },
    )?;
    check!(eq; result.is_some(), completes);
    Ok(())
}

mod replay;
