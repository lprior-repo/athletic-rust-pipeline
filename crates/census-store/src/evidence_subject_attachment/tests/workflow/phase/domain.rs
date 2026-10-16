use super::meter::{object_position, position, Meter};
use super::model::Record;
use crate::public_identity_tests::support::TestResult;
use census_domain::evidence_subject_attachment::*;

pub(super) fn command(record: &Record, meter: &mut Meter) -> TestResult {
    let command = &record.command;
    let context = &command.reviewed_context;
    check!(eq; command.subjects.len(), 1);
    check!(eq; context.facts.len(), 2);
    check!(eq; context.corroboration.len(), 1);
    check!(eq; context.complete_scope_records.len(), 1);
    check!(eq; context.accepted_target_subjects.len(), 1);
    let n = context.objects.len();
    meter.charge("domain command schema", 1)?;
    meter.charge(
        "domain combined reference admission",
        n.checked_add(6).ok_or("reference count overflow")?,
    )?;
    meter.singles(
        "domain sorted uniqueness",
        n.checked_add(5).ok_or("uniqueness count overflow")?,
    )?;
    meter.singles("domain nonempty references", n)?;
    let subject = command
        .subjects
        .as_slice()
        .first()
        .ok_or("subject absent")?;
    let mut refs = vec![
        &context.accepted_target_witness,
        &subject.capture,
        &subject.body,
        &subject.interpretation,
        &subject.locator,
        &subject.subject,
        &subject.context,
    ];
    refs.extend(context.accepted_target_subjects.as_slice());
    refs.extend(context.complete_scope_records.as_slice());
    for fact in context.facts.as_slice() {
        refs.extend([&fact.fact, &fact.subject, &fact.publisher_lineage]);
    }
    check!(eq; refs.len(), 15);
    for digest in refs {
        meter.singles(
            "domain object resolution",
            object_position(record.references(), digest)?,
        )?;
    }
    meter.singles("domain paired fact and subject resolution", 5)?;
    if matches!(command.action, AttachmentActionV1::Attach { .. }) {
        meter.charge("domain complete paired coverage", 1)?;
        let pair = context
            .corroboration
            .as_slice()
            .first()
            .ok_or("pair absent")?;
        let facts = context
            .facts
            .as_slice()
            .iter()
            .map(|fact| fact.fact.clone())
            .collect::<Vec<_>>();
        meter.singles(
            "domain coverage source fact",
            position(&facts, &pair.subject_fact)?,
        )?;
    }
    Ok(())
}

pub(super) fn record(record: &Record, meter: &mut Meter) -> TestResult {
    meter.singles("domain record admission and measure", 2)?;
    command(record, meter)?;
    meter.singles("domain three digest groups", 6)
}

pub(super) fn inventory(count: usize, meter: &mut Meter) -> TestResult {
    meter.charge("domain inventory reference traversal", count)?;
    meter.singles("domain inventory measure and hash", 2)
}

pub(super) fn projection(records: &[&Record], meter: &mut Meter) -> TestResult {
    meter.singles("Store projected receipt construction", records.len())?;
    for record in records {
        self::record(record, meter)?;
        meter.singles("domain receipt measure and origin", 2)?;
    }
    let mut active = Vec::new();
    for (index, record) in records.iter().enumerate() {
        inventory(index, meter)?;
        let prior = records.get(..index).ok_or("projection prefix absent")?;
        for old in prior {
            meter.charge("domain latest-chain operation", 1)?;
            if old.command.target.public_id == record.command.target.public_id {
                meter.charge("domain exact subject chain", 1)?;
                check!(eq; old.command.subjects, record.command.subjects);
            }
        }
        match &record.command.action {
            AttachmentActionV1::Revoke { .. } => {
                meter.charge("domain exact predecessor", 1)?;
                let old = prior.last().ok_or("revocation prior absent")?;
                retained(old, record, meter)?;
                *active.last_mut().ok_or("revocation state absent")? = false;
                active.push(false);
            }
            AttachmentActionV1::Attach {
                prior: AttachmentPredecessorV1::First,
            } => {
                for (old, is_active) in prior.iter().zip(&active) {
                    meter.charge("domain active overlap", 1)?;
                    if *is_active {
                        for left in old.command.subjects.as_slice() {
                            for right in record.command.subjects.as_slice() {
                                meter.charge("domain overlap subject", 1)?;
                                check!(ne; left.subject, right.subject);
                            }
                        }
                    }
                }
                active.push(true);
            }
            AttachmentActionV1::Attach {
                prior: AttachmentPredecessorV1::After(_),
            } => return Err("phase oracle covers fresh First after revocation only".into()),
        }
    }
    inventory(records.len(), meter)
}

fn retained(old: &Record, new: &Record, meter: &mut Meter) -> TestResult {
    let old = &old.command.reviewed_context;
    let new = &new.command.reviewed_context;
    retain(old.objects.as_slice(), new.objects.as_slice(), meter)?;
    retain(old.facts.as_slice(), new.facts.as_slice(), meter)?;
    retain(
        old.corroboration.as_slice(),
        new.corroboration.as_slice(),
        meter,
    )?;
    retain(
        old.complete_scope_records.as_slice(),
        new.complete_scope_records.as_slice(),
        meter,
    )?;
    retain(
        old.accepted_target_subjects.as_slice(),
        new.accepted_target_subjects.as_slice(),
        meter,
    )
}

fn retain<T: PartialEq>(old: &[T], new: &[T], meter: &mut Meter) -> TestResult {
    for value in old {
        meter.singles(
            "domain complete retained predecessor context",
            position(new, value)?,
        )?;
    }
    Ok(())
}
