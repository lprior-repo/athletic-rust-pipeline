use super::*;
use census_store::{ResultSchoolCorrespondenceStoreError, StoreError};

#[test]
fn genuine_tfrrs_neighboring_dates_cannot_be_swapped_in_school_proposals() -> TestResult {
    let (_directory, store, input) = fixture::install()?;
    let before = setup::state(&store)?;
    let originals = fixture::originals(&store, &input)?;
    for variant in 0..5 {
        let (index, scope) = altered_scope(variant)?;
        let mut grant = AttachmentReplayGrantV1::new();
        let result = fixture::proposal_with_grant(&store, &input, index, scope, &mut grant);
        refused_source(
            result,
            "TFRRS school date sport category year differs from original row",
        )?;
        poisoned(&grant)?;
        check!(eq; setup::state(&store)?, before);
        check!(eq; fixture::originals(&store, &input)?, originals);
    }
    Ok(())
}

fn altered_scope(variant: u8) -> TestResult<(usize, ResultSchoolScopeV1)> {
    let mut scope = scope_for(0)?;
    match variant {
        0 => return Ok((0, scope_for(1)?)),
        1 => return Ok((1, scope_for(0)?)),
        2 => scope.sport = Sport::OutdoorTrack,
        3 => scope.category = Gender::Girls,
        4 => scope.year = SchoolYear::new(2026).ok_or("changed school year")?,
        _ => return Err("changed scope index".into()),
    }
    Ok((0, scope))
}

#[test]
fn tfrrs_school_rehashed_unknown_and_milesplit_parser_contracts_refuse_at_prepare() -> TestResult {
    let (_directory, store, input) = fixture::install()?;
    let proposal = fixture::proposal(&store, &input, 0, scope_for(0)?)?;
    let before = setup::state(&store)?;
    let originals = fixture::originals(&store, &input)?;
    let mut unknown = RESULT_SCHOOL_TFRRS_PARSER_CONTRACT_V1.to_vec();
    unknown.push(b' ');
    for (index, bytes, detail) in [
        (0, unknown.as_slice(), "school parser contract"),
        (
            1,
            RESULT_SCHOOL_PARSER_CONTRACT_V1,
            "row interpretation binding",
        ),
    ] {
        changed_contract(&store, &proposal, index, bytes, detail)?;
        check!(eq; setup::state(&store)?, before);
        check!(eq; fixture::originals(&store, &input)?, originals);
    }
    Ok(())
}

fn changed_contract(
    store: &Store,
    proposal: &ResultSchoolSourceProposalV1,
    index: usize,
    bytes: &[u8],
    expected: &str,
) -> TestResult {
    let original = &proposal.claim().statement.parser_contract;
    let replacement = AttachmentObjectReferenceV1 {
        digest: digest(bytes)?,
        bytes: u64::try_from(bytes.len())?,
    };
    let mut source = proposal.claim().clone();
    source.statement.parser_contract = replacement.clone();
    let mut evidence = source.evidence.as_slice().to_vec();
    let found = evidence
        .iter_mut()
        .find(|item| *item == original)
        .ok_or("parser evidence")?;
    *found = replacement.clone();
    evidence.sort_by(|left, right| left.digest.cmp(&right.digest));
    source.evidence = list(evidence)?;
    let context = proposal
        .borrowed_context(&mut AttachmentReplayGrantV1::new())?
        .into_iter()
        .map(|item| match item {
            AttachmentContextObjectV1::Inline { reference, .. } if reference == original => {
                AttachmentContextObjectV1::Inline {
                    reference: &replacement,
                    bytes,
                }
            }
            item => item,
        })
        .collect::<Vec<_>>();
    let inspection =
        store.inspect_result_school_correspondence_v1(AttachmentReplayGrantV1::new())?;
    let command = review::command(
        &inspection,
        &source,
        &format!("controlled-tfrrs-parser-refusal-{index}"),
        ResultSchoolActionV1::Accept {
            prior: ResultSchoolPredecessorV1::First,
        },
    )?;
    match store.prepare_result_school_correspondence_v1(
        command,
        ResultSchoolCorrespondencePreparationV1::New {
            inspection,
            context: &context,
        },
    ) {
        Err(StoreError::ResultSchoolCorrespondence(
            ResultSchoolCorrespondenceStoreError::SourceMismatch(detail),
        )) => check!(eq; detail, expected),
        Err(error) => return Err(format!("unexpected parser refusal: {error}").into()),
        Ok(_) => return Err("changed parser contract prepared".into()),
    }
    Ok(())
}

#[test]
fn milesplit_school_rehashed_tfrrs_parser_contract_refuses_at_prepare() -> TestResult {
    let (_directory, store, input) = setup::install()?;
    attachment::apply(&store, &input)?;
    setup::enroll(&store)?;
    let proposal = review::proposal(&store, &input)?;
    let before = setup::state(&store)?;
    let originals = setup::originals(&store, &input)?;
    changed_contract(
        &store,
        &proposal,
        2,
        RESULT_SCHOOL_TFRRS_PARSER_CONTRACT_V1,
        "row interpretation binding",
    )?;
    check!(eq; setup::state(&store)?, before);
    check!(eq; setup::originals(&store, &input)?, originals);
    Ok(())
}

#[test]
fn test_reviewed_rehashed_partial_quote_and_changed_day_fail_current_and_frozen_recheck(
) -> TestResult {
    for index in 0..2 {
        let (_directory, store, input) = fixture::install()?;
        let proposal = fixture::proposal(&store, &input, 0, scope_for(0)?)?;
        let originals = fixture::originals(&store, &input)?;
        let mut source = proposal.claim().clone();
        let detail = if index == 0 {
            source.statement.statement_text = text("Columbus")?;
            source.statement.statement_length = 8;
            source.statement.statement_digest = digest(b"Columbus")?;
            "reviewed school differs from original source recheck"
        } else {
            source.target.day = scope_for(1)?.day;
            "TFRRS school date sport category year differs from original row"
        };
        commands::accept(&store, &proposal, &source, index)?;
        let before = setup::state(&store)?;
        let proof = observe::freeze(&store)?;
        let mut current_grant = AttachmentReplayGrantV1::new();
        refused_source(
            observe::current_with_grant(&store, &mut current_grant),
            detail,
        )?;
        poisoned(&current_grant)?;
        let mut frozen_grant = AttachmentReplayGrantV1::new();
        refused_source(
            observe::frozen_with_grant(&proof, &input, &mut frozen_grant),
            detail,
        )?;
        poisoned(&frozen_grant)?;
        check!(eq; fixture::originals(&store, &input)?, originals);
        check!(eq; setup::state(&store)?, before);
    }
    Ok(())
}

fn refused_source<T>(result: crate::CrawlResult<T>, expected: &str) -> TestResult {
    match result {
        Err(crate::CrawlError::Store(StoreError::Refused { detail })) => {
            check!(eq; detail, expected)
        }
        Err(error) => return Err(format!("unexpected strict source refusal: {error}").into()),
        Ok(_) => return Err("altered TFRRS school source was accepted".into()),
    }
    Ok(())
}

fn poisoned(grant: &AttachmentReplayGrantV1) -> TestResult {
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}
