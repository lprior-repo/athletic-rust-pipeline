mod scope;

use super::super::source::{require_originals, InstalledSource, TestResult};
use super::revocation::{apply_calendar_action, enroll_calendar, prepare_action, require_current};
use census_crawl::participation_evidence::result_ownership::calendar_correspondence::ResultCalendarSourceProposalV1;
use census_domain::result_calendar_correspondence::{
    ResultCalendarActionV1, ResultCalendarPredecessorV1,
};
use census_store::{ResultCalendarCorrespondenceStoreError, StoreError};

fn setup() -> TestResult<(InstalledSource, ResultCalendarSourceProposalV1)> {
    let (installed, points, _accepted) = super::install_reviewed()?;
    enroll_calendar(
        &installed.store,
        "controlled-calendar-refusal-enrollment",
        "Explicit calendar capability for public refusal scenarios",
    )?;
    let proposal = super::proposal::may15_proposal(&installed, &points)?;
    Ok((installed, proposal))
}

#[test]
fn changed_opaque_approval_refuses_without_mutation_then_exact_review_succeeds() -> TestResult {
    let (installed, proposal) = setup()?;
    let operation = "controlled-calendar-approval-refusal";
    let rationale = "Exact May15 review after rejected opaque approval";
    let (command, prepared) = prepare_action(
        &installed.store,
        &proposal,
        operation,
        ResultCalendarActionV1::Accept {
            prior: ResultCalendarPredecessorV1::First,
        },
        rationale,
    )?;
    let mut approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    *approved
        .first_mut()
        .ok_or("opaque calendar approval absent")? ^= 1;
    let before = super::protected_state(&installed.store)?;
    let error = installed
        .store
        .apply_result_calendar_correspondence_v1(prepared, &approved, &digest)
        .err()
        .ok_or("changed opaque approval was accepted")?;
    check!(matches!(
        error,
        StoreError::ResultCalendarCorrespondence(
            ResultCalendarCorrespondenceStoreError::ApprovalMismatch
        )
    ));
    check!(eq; super::protected_state(&installed.store)?, before);
    require_current(&installed.store, 0, 0, 0)?;
    require_originals(&installed.store, &installed.source_records)?;
    let (accepted, receipt) = apply_calendar_action(
        &installed.store,
        &proposal,
        operation,
        ResultCalendarActionV1::Accept {
            prior: ResultCalendarPredecessorV1::First,
        },
        rationale,
    )?;
    check!(eq; accepted, command);
    let (_, records) = require_current(&installed.store, 1, 1, 1)?;
    let [record] = records.as_slice() else {
        return Err("exactly one approved correspondence required".into());
    };
    check!(eq; record.command, command);
    check!(eq; record.record_digest.as_str(), receipt.digest.as_str());
    require_originals(&installed.store, &installed.source_records)
}

#[test]
fn stale_prepared_review_cannot_replace_the_committed_calendar_winner() -> TestResult {
    let (installed, proposal) = setup()?;
    let (_, pending) = prepare_action(
        &installed.store,
        &proposal,
        "controlled-calendar-pending-a",
        ResultCalendarActionV1::Accept {
            prior: ResultCalendarPredecessorV1::First,
        },
        "Pending exact review before another approved operation",
    )?;
    let approved = pending.approval_bytes().to_vec();
    let digest = pending.approval_digest().clone();
    let (winner, receipt) = apply_calendar_action(
        &installed.store,
        &proposal,
        "controlled-calendar-winner-b",
        ResultCalendarActionV1::Accept {
            prior: ResultCalendarPredecessorV1::First,
        },
        "Exact review committed before the pending operation",
    )?;
    let before = super::protected_state(&installed.store)?;
    let (inventory, records) = require_current(&installed.store, 1, 1, 1)?;
    let error = installed
        .store
        .apply_result_calendar_correspondence_v1(pending, &approved, &digest)
        .err()
        .ok_or("stale prepared calendar review was accepted")?;
    check!(matches!(
        error,
        StoreError::RevisionMoved { .. }
            | StoreError::ResultCalendarCorrespondence(
                ResultCalendarCorrespondenceStoreError::InputMoved
            )
    ));
    check!(eq; super::protected_state(&installed.store)?, before);
    let (after_inventory, after_records) = require_current(&installed.store, 1, 1, 1)?;
    check!(eq; after_inventory, inventory);
    check!(eq; after_records, records);
    let [record] = after_records.as_slice() else {
        return Err("committed calendar winner disappeared".into());
    };
    check!(eq; record.command, winner);
    check!(eq; record.record_digest.as_str(), receipt.digest.as_str());
    require_originals(&installed.store, &installed.source_records)
}

#[test]
fn foreign_store_cannot_apply_a_prepared_calendar_review_or_change_either_owner() -> TestResult {
    let (owner, proposal) = setup()?;
    let (foreign, _) = setup()?;
    let operation = "controlled-calendar-foreign-owner";
    let rationale = "Exact review belongs only to its inspected store";
    let (command, prepared) = prepare_action(
        &owner.store,
        &proposal,
        operation,
        ResultCalendarActionV1::Accept {
            prior: ResultCalendarPredecessorV1::First,
        },
        rationale,
    )?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    let owner_before = super::protected_state(&owner.store)?;
    let foreign_before = super::protected_state(&foreign.store)?;
    let owner_inventory = require_current(&owner.store, 0, 0, 0)?;
    let foreign_inventory = require_current(&foreign.store, 0, 0, 0)?;
    let error = foreign
        .store
        .apply_result_calendar_correspondence_v1(prepared, &approved, &digest)
        .err()
        .ok_or("foreign store applied another owner's calendar review")?;
    check!(
        matches!(error, StoreError::RevisionOriginMismatch { operation: actual }
        if actual == operation)
    );
    check!(eq; super::protected_state(&owner.store)?, owner_before);
    check!(eq; super::protected_state(&foreign.store)?, foreign_before);
    check!(eq; require_current(&owner.store, 0, 0, 0)?, owner_inventory);
    check!(eq; require_current(&foreign.store, 0, 0, 0)?, foreign_inventory);
    require_originals(&owner.store, &owner.source_records)?;
    require_originals(&foreign.store, &foreign.source_records)?;
    let (accepted, receipt) = apply_calendar_action(
        &owner.store,
        &proposal,
        operation,
        ResultCalendarActionV1::Accept {
            prior: ResultCalendarPredecessorV1::First,
        },
        rationale,
    )?;
    check!(eq; accepted, command);
    let (_, records) = require_current(&owner.store, 1, 1, 1)?;
    let [record] = records.as_slice() else {
        return Err("inspected owner lost its exact accepted review".into());
    };
    check!(eq; record.command, command);
    check!(eq; record.record_digest.as_str(), receipt.digest.as_str());
    check!(eq; super::protected_state(&foreign.store)?, foreign_before);
    check!(eq; require_current(&foreign.store, 0, 0, 0)?, foreign_inventory);
    require_originals(&owner.store, &owner.source_records)?;
    require_originals(&foreign.store, &foreign.source_records)
}
