use super::*;
use crate::evidence_subject_attachment::*;
use crate::model::*;
use crate::public_identity_history::PublicAthleteId;
mod fixture;
use fixture::*;
type TestResult<T = (), E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

#[test]
fn reviewed_day_revocation_withholds_current_and_preserves_original_acceptance() -> TestResult {
    let accepted = record(command("calendar-accept")?)?;
    let initial = accepted.command.expected_inventory.clone();
    let after_accept = next(&accepted)?;
    let mut revoke = command("calendar-revoke")?;
    revoke.source = accepted.command.source.clone();
    revoke.expected_inventory = after_accept.clone();
    revoke.action = ResultCalendarActionV1::Revoke {
        prior: ResultCalendarRecordReferenceV1 {
            operation: accepted.command.operation.clone(),
            digest: accepted.record_digest.clone(),
        },
    };
    let revoked = record(revoke)?;
    let after_revoke = next(&revoked)?;
    let original_bytes = crate::model::serialized_bytes(&accepted)?;
    let records = [accepted, revoked];
    let receipts = records.iter().map(receipt).collect::<Vec<_>>();
    let old = replay_result_calendar_records_v1(
        &records[0].command.store,
        &initial,
        &records[..1],
        &receipts[..1],
        &after_accept,
        &mut AttachmentReplayGrantV1::new(),
    )?;
    check!(eq; old.active_record_indices().collect::<Vec<_>>(), [0]);
    let current = replay_result_calendar_records_v1(
        &records[0].command.store,
        &initial,
        &records,
        &receipts,
        &after_revoke,
        &mut AttachmentReplayGrantV1::new(),
    )?;
    check!(eq; current.active_record_indices().collect::<Vec<_>>(), Vec::<usize>::new());
    check!(eq; current.records(), &records);
    let decoded =
        decode_result_calendar_record_v1(&original_bytes, &mut AttachmentReplayGrantV1::new())?;
    check!(eq; decoded, records[0]);
    let mut duplicate = records.to_vec();
    duplicate.push(records[0].clone());
    let duplicate_receipts = duplicate.iter().map(receipt).collect::<Vec<_>>();
    check!(matches!(
        replay_result_calendar_records_v1(
            &records[0].command.store,
            &initial,
            &duplicate,
            &duplicate_receipts,
            &after_revoke,
            &mut AttachmentReplayGrantV1::new()
        ),
        Err(ResultCalendarCorrespondenceError::InventoryMismatch
            | ResultCalendarCorrespondenceError::PredecessorConflict)
    ));
    Ok(())
}

#[test]
fn wrong_day_year_and_missing_required_original_proof_refuse() -> TestResult {
    let mut wrong_year = command("wrong-year")?;
    wrong_year.source.year = SchoolYear::new(2026).ok_or("year")?;
    let mut missing = command("missing-proof")?;
    missing.source.evidence =
        AttachmentListV1::new(vec![missing.source.result.result_root.clone()])?;
    for value in [wrong_year, missing] {
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            CheckedResultCalendarCommandV1::check(value, &mut grant),
            Err(ResultCalendarCorrespondenceError::Invalid(_))
        ));
        check!(grant.ensure_available().is_err());
    }
    Ok(())
}

#[test]
fn checked_command_rejects_foreign_grant_without_consuming_original() -> TestResult {
    let mut original = AttachmentReplayGrantV1::new();
    let checked = CheckedResultCalendarCommandV1::check(command("original-grant")?, &mut original)?;
    let mut foreign = AttachmentReplayGrantV1::new();
    check!(matches!(
        checked.command(&mut foreign),
        Err(ResultCalendarCorrespondenceError::Grant(_))
    ));
    check!(foreign.ensure_available().is_err());
    check!(eq; checked.command(&mut original)?.operation.as_str(), "original-grant");
    Ok(())
}

#[test]
fn changed_or_noncanonical_required_record_never_replays_as_accepted() -> TestResult {
    let value = record(command("canonical-record")?)?;
    let mut bytes = crate::model::serialized_bytes(&value)?;
    bytes.push(b'\n');
    let mut grant = AttachmentReplayGrantV1::new();
    check!(decode_result_calendar_record_v1(&bytes, &mut grant).is_err());
    check!(grant.ensure_available().is_err());
    let mut changed = value;
    changed.command.source.day = ProfileDate::parse("2026-05-16")?;
    let changed = crate::model::serialized_bytes(&changed)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        decode_result_calendar_record_v1(&changed, &mut grant),
        Err(ResultCalendarCorrespondenceError::Invalid(_))
    ));
    check!(grant.ensure_available().is_err());
    Ok(())
}
