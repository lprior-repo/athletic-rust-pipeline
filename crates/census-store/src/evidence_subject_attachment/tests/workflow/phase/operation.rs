use super::meter::{depth, Meter};
use super::model::Record;
use super::{domain, ledger, model};
use crate::evidence_subject_attachment::AttachmentInspectionRequestV1;
use crate::public_identity_tests::support::TestResult;
use census_domain::evidence_subject_attachment::{AttachmentActionV1, AttachmentPredecessorV1};

pub(super) fn inspect(
    records: &[&Record],
    request: &AttachmentInspectionRequestV1,
    meter: &mut Meter,
) -> TestResult {
    meter.charge("inspection format", 1)?;
    meter.charge("inspection capture helper", 3)?;
    meter.singles("inspection encoded capture", 3)?;
    meter.charge("inspection token helper", 2)?;
    meter.singles("inspection raw token", 3)?;
    meter.charge("inspection StoreIdentity", 1)?;
    ledger::load(records, meter)?;
    meter.charge("inspection head", 2)?;
    match request {
        AttachmentInspectionRequestV1::Attach {
            prior: AttachmentPredecessorV1::First,
            ..
        } => meter.singles("inspection accepted target one claim and witness", 11)?,
        AttachmentInspectionRequestV1::Revoke { prior } => {
            let position = records
                .iter()
                .position(|record| record.command.operation == prior.operation)
                .and_then(|index| index.checked_add(1))
                .ok_or("inspection predecessor absent")?;
            meter.singles("inspection exact predecessor lookup", position)?;
        }
        AttachmentInspectionRequestV1::Attach {
            prior: AttachmentPredecessorV1::After(_),
            ..
        } => return Err("phase oracle uses fresh First chains".into()),
    }
    Ok(())
}

pub(super) fn prepare(prior: &[&Record], record: &Record, meter: &mut Meter) -> TestResult {
    meter.charge("preparation StoreIdentity", 1)?;
    ledger::refresh(prior, meter)?;
    domain::command(record, meter)?;
    meter.charge("command proof issuance", 1)?;
    meter.charge("checked approval origin", 1)?;
    meter.singles("approval measure and encode", 2)?;
    meter.singles("preparation operation lookup", prior.len())?;
    current_fences(meter)?;
    for reference in &record.issued {
        meter.singles(
            "issued input declaration",
            depth(record.references(), &reference.digest)?,
        )?;
    }
    model::introductions(prior, record, meter)?;
    meter.charge("command to draft origin", 1)?;
    meter.singles("preparation three digest groups", 6)?;
    proposed(prior, record, meter)?;
    meter.charge("restore previous ledger owner", 1)?;
    meter.singles("restore collection origin and extracted owner issuance", 2)
}

pub(super) fn apply(prior: &[&Record], record: &Record, meter: &mut Meter) -> TestResult {
    meter.singles("apply approval and StoreIdentity", 2)?;
    ledger::refresh(prior, meter)?;
    meter.singles("apply operation lookup", prior.len())?;
    current_fences(meter)?;
    if matches!(record.command.action, AttachmentActionV1::Attach { .. }) {
        meter.singles("apply accepted target one claim and full comparison", 8)?;
    }
    meter.charge("checked commit time origin", 1)?;
    meter.charge("final commit time", 1)?;
    meter.singles("final record digest", 2)?;
    meter.charge("checked record encode origin", 1)?;
    domain::record_shell(meter)?;
    meter.singles("final record encode measure and serialization", 2)?;
    meter.charge("checked roundtrip origin", 1)?;
    meter.charge("final record decode", 1)?;
    meter.charge("complete decoded record equality", 1)?;
    domain::record_shell(meter)?;
    meter.charge("roundtrip record proof issuance", 1)?;
    proposed(prior, record, meter)?;
    meter.singles("inline journal key and envelope object", 4)?;
    meter.charge("prospective physical inline row", 1)?;
    meter.charge("protected receipt frame", 2)
}

fn current_fences(meter: &mut Meter) -> TestResult {
    meter.charge("current revision and inventory fence", 1)?;
    meter.charge("current public head", 2)?;
    meter.charge("current input format", 1)?;
    meter.charge("current capture helper", 3)?;
    meter.charge("current semantic input predicate", 1)?;
    meter.charge("current full token helper", 2)?;
    meter.charge("current full token equality", 1)
}

fn proposed(prior: &[&Record], record: &Record, meter: &mut Meter) -> TestResult {
    model::closure(prior, record, meter)?;
    meter.singles("prospective record and collection origin", 2)?;
    let mut prospective = prior.to_vec();
    prospective.push(record);
    domain::inventory(prospective.len(), meter)?;
    ledger::fences(&prospective, record, meter)?;
    domain::projection(&prospective, meter)?;
    ledger::history(&prospective, record, meter)
}
