use super::fixture::*;
use crate::public_identity_tests::support::{retained_state, TestResult};
use census_domain::evidence_subject_attachment::*;

const EMPTY_LEDGER_STEPS: usize = 8;
const ONE_CLAIM_INSPECTION_STEPS: usize = 34;
const SIXTEEN_SORTED_LOOKUP_DEPTHS: [usize; 16] = [5, 4, 3, 4, 2, 4, 3, 4, 1, 4, 3, 4, 2, 4, 3, 4];
const TWO_SORTED_FACT_DEPTHS: [usize; 2] = [2, 1];

fn sum(values: impl IntoIterator<Item = usize>) -> TestResult<usize> {
    values.into_iter().try_fold(0usize, |total, value| {
        total
            .checked_add(value)
            .ok_or_else(|| "accounting sum overflow".into())
    })
}

fn product(left: usize, right: usize) -> TestResult<usize> {
    left.checked_mul(right)
        .ok_or_else(|| "accounting product overflow".into())
}

fn position(
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    digest: &AttachmentDigestV1,
) -> TestResult<usize> {
    command
        .reviewed_context
        .objects
        .as_slice()
        .iter()
        .position(|object| &object.digest == digest)
        .and_then(|index| index.checked_add(1))
        .ok_or_else(|| "accounting reference absent".into())
}

fn depth(one_based: usize) -> TestResult<usize> {
    let index = one_based
        .checked_sub(1)
        .ok_or("lookup position underflow")?;
    SIXTEEN_SORTED_LOOKUP_DEPTHS
        .get(index)
        .copied()
        .ok_or_else(|| "lookup depth absent".into())
}

fn command_steps(command: &RetainedEvidenceSubjectAttachmentCommandV1) -> TestResult<usize> {
    check!(eq; command.subjects.len(), 1);
    check!(eq; command.reviewed_context.facts.len(), 2);
    check!(eq; command.reviewed_context.corroboration.len(), 1);
    check!(eq; command.reviewed_context.complete_scope_records.len(), 1);
    check!(eq; command.reviewed_context.accepted_target_subjects.len(), 1);
    let subject = command
        .subjects
        .as_slice()
        .first()
        .ok_or("accounting subject absent")?;
    let reviewed = &command.reviewed_context;
    let mut refs = vec![
        &reviewed.accepted_target_witness,
        &subject.capture,
        &subject.body,
        &subject.interpretation,
        &subject.locator,
        &subject.subject,
        &subject.context,
    ];
    refs.extend(reviewed.accepted_target_subjects.as_slice());
    refs.extend(reviewed.complete_scope_records.as_slice());
    for fact in reviewed.facts.as_slice() {
        refs.extend([&fact.fact, &fact.subject, &fact.publisher_lineage]);
    }
    check!(eq; refs.len(), 15);
    let resolution = sum(refs
        .into_iter()
        .map(|digest| depth(position(command, digest)?))
        .collect::<TestResult<Vec<_>>>()?)?;
    let pair = reviewed
        .corroboration
        .as_slice()
        .first()
        .ok_or("accounting pair absent")?;
    let source_fact_index = reviewed
        .facts
        .as_slice()
        .iter()
        .position(|fact| fact.fact == pair.subject_fact)
        .ok_or("accounting source fact absent")?;
    let source_fact_depth = TWO_SORTED_FACT_DEPTHS
        .get(source_fact_index)
        .copied()
        .ok_or("accounting source fact depth absent")?;
    let n = reviewed.objects.len();
    sum([
        1,
        sum([n, 6])?,
        sum([n, 5])?,
        n,
        resolution,
        5,
        1,
        source_fact_depth,
    ])
}

fn insertion_comparisons(
    issued: &[AttachmentObjectReferenceV1],
    context: &Context,
) -> TestResult<usize> {
    let order = issued
        .iter()
        .chain(context.objects.iter().map(|object| &object.reference))
        .collect::<Vec<_>>();
    let mut comparisons = Vec::new();
    for (index, next) in order.iter().enumerate() {
        let prefix = order
            .get(..index)
            .ok_or("accounting insertion prefix absent")?;
        let less = prefix
            .iter()
            .filter(|prior| prior.digest < next.digest)
            .count();
        let first_greater = if prefix.iter().any(|prior| prior.digest > next.digest) {
            1
        } else {
            0
        };
        comparisons.push(sum([less, first_greater])?);
    }
    sum(comparisons)
}

struct FirstRecordWork {
    command: usize,
    context_closure: usize,
    input_fences: usize,
    historical_witness: usize,
    issued_declarations: usize,
}

fn first_record_work(
    command: &RetainedEvidenceSubjectAttachmentCommandV1,
    issued: &[AttachmentObjectReferenceV1],
) -> TestResult<FirstRecordWork> {
    let [group, witness, capture, token]: &[AttachmentObjectReferenceV1; 4] = issued.try_into()?;
    let g = position(command, &group.digest)?;
    let w = position(command, &witness.digest)?;
    let c = position(command, &capture.digest)?;
    let t = position(command, &token.digest)?;
    let n = command.reviewed_context.objects.len();
    check!(eq; n, 16);
    check!(eq; sum(SIXTEEN_SORTED_LOOKUP_DEPTHS)?, 54);
    Ok(FirstRecordWork {
        command: command_steps(command)?,
        context_closure: sum([product(2, 54)?, product(n, 6)?])?,
        input_fences: sum([16, c, t, depth(c)?, depth(t)?])?,
        historical_witness: sum([30, depth(w)?, depth(g)?, depth(c)?, depth(t)?, g, c, t])?,
        issued_declarations: sum([g, w, c, t])?,
    })
}

#[test]
fn first_record_inspection_preparation_and_held_read_match_independently_counted_source_work(
) -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("named-work-oracle")?;
    let inspection = fixture.inspect()?;
    check!(eq; inspection.grant.as_ref().ok_or("original inspection grant absent")?.usage().traversal_steps(), ONE_CLAIM_INSPECTION_STEPS);
    let issued = inspection.issued_objects().cloned().collect::<Vec<_>>();
    let command = context.command("named-work-oracle", &inspection)?;
    let work = first_record_work(&command, &issued)?;
    let introduction_steps = sum([
        product(16, 3)?,
        context.objects.len(),
        insertion_comparisons(&issued, &context)?,
    ])?;
    let expected_prepared = sum([
        ONE_CLAIM_INSPECTION_STEPS,
        sum([1, EMPTY_LEDGER_STEPS])?,
        sum([work.command, 2])?,
        sum([11, work.issued_declarations])?,
        introduction_steps,
        6,
        work.context_closure,
        3,
        work.input_fences,
        sum([work.command, 16])?,
        work.historical_witness,
    ])?;
    let prepared = prepare(&fixture.store, inspection, command, &context)?;
    check!(eq; prepared.grant.usage().traversal_steps(), expected_prepared);
    written(apply(&fixture.store, prepared)?)?;
    let before = retained_state(&fixture.store)?;
    let expected_read = sum([
        1,
        7,
        34,
        sum([work.command, 12])?,
        work.context_closure,
        7,
        3,
        sum([work.command, 16])?,
        work.input_fences,
        work.historical_witness,
    ])?;
    let held = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let read = held.read_evidence_subject_attachment_references_v1(&mut grant)?;
    check!(eq; read.records().len(), 1);
    check!(eq; grant.usage().traversal_steps(), expected_read);
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}
