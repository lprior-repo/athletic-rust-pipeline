use super::super::protected_state;
use super::{
    apply_calendar_action, prepare_action, require_current, require_originals, setup, TestResult,
};
use census_crawl::participation_evidence::result_ownership::calendar_correspondence::ResultCalendarSourceProposalV1;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{Gender, ProfileDate, SchoolYear, Sport};
use census_domain::result_calendar_correspondence::{
    ResultCalendarActionV1, ResultCalendarCorrespondenceError, ResultCalendarDigestV1,
    ResultCalendarPredecessorV1, ResultCalendarReviewCommandV1,
};
use census_store::{
    ResultCalendarCorrespondencePreparationV1, ResultCalendarCorrespondenceStoreError, Store,
    StoreError,
};

#[derive(Debug)]
enum Fault {
    Day,
    Sport,
    Category,
    Year,
    Row,
    DocumentaryRoot,
}

fn changed(
    command: &ResultCalendarReviewCommandV1,
    fault: Fault,
) -> TestResult<ResultCalendarReviewCommandV1> {
    let mut candidate = command.clone();
    match fault {
        Fault::Day => candidate.source.day = ProfileDate::parse("2026-05-16")?,
        Fault::Sport => candidate.source.sport = Sport::CrossCountry,
        Fault::Category => candidate.source.category = Gender::Girls,
        Fault::Year => candidate.source.year = SchoolYear::new(2026).ok_or("academic year")?,
        Fault::Row => {
            candidate.source.result.row_digest = ResultCalendarDigestV1::parse(
                "0000000000000000000000000000000000000000000000000000000000000001",
            )?
        }
        Fault::DocumentaryRoot => {
            candidate.source.calendar.documentary_root =
                candidate.source.result.result_root.clone();
        }
    }
    Ok(candidate)
}

fn refuse(
    store: &Store,
    proposal: &ResultCalendarSourceProposalV1,
    command: ResultCalendarReviewCommandV1,
) -> TestResult {
    let inspection =
        store.inspect_result_calendar_correspondence_v1(AttachmentReplayGrantV1::new())?;
    let context = proposal.borrowed_context(&mut AttachmentReplayGrantV1::new())?;
    let refused = store.prepare_result_calendar_correspondence_v1(
        command,
        ResultCalendarCorrespondencePreparationV1::New {
            inspection,
            context: &context,
        },
    );
    let error = refused.err().ok_or("changed source scope was admitted")?;
    check!(matches!(
        error,
        StoreError::ResultCalendarCorrespondence(
            ResultCalendarCorrespondenceStoreError::SourceMismatch(_)
                | ResultCalendarCorrespondenceStoreError::Domain(
                    ResultCalendarCorrespondenceError::Invalid(_)
                )
        )
    ));
    Ok(())
}

#[test]
fn changed_source_scope_refuses_without_mutation_and_exact_original_remains_acceptable(
) -> TestResult {
    let (installed, proposal) = setup()?;
    let operation = "controlled-calendar-scope-refusals";
    let rationale = "Only original May15 source scope is approved";
    let (command, prepared) = prepare_action(
        &installed.store,
        &proposal,
        operation,
        ResultCalendarActionV1::Accept {
            prior: ResultCalendarPredecessorV1::First,
        },
        rationale,
    )?;
    drop(prepared);
    let before = protected_state(&installed.store)?;
    let inventory = require_current(&installed.store, 0, 0, 0)?;
    for fault in [
        Fault::Day,
        Fault::Sport,
        Fault::Category,
        Fault::Year,
        Fault::Row,
        Fault::DocumentaryRoot,
    ] {
        let candidate = changed(&command, fault)?;
        check!(candidate.source != command.source);
        refuse(&installed.store, &proposal, candidate)?;
        check!(eq; protected_state(&installed.store)?, before);
        check!(eq; require_current(&installed.store, 0, 0, 0)?, inventory);
        require_originals(&installed.store, &installed.source_records)?;
    }
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
        return Err("exact source scope did not produce one accepted correspondence".into());
    };
    check!(eq; record.command, command);
    check!(eq; record.record_digest.as_str(), receipt.digest.as_str());
    require_originals(&installed.store, &installed.source_records)
}
