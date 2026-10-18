use super::*;
use census_store::{
    PreparedResultSchoolCorrespondenceV1, ResultSchoolCorrespondencePreparationV1,
    ResultSchoolCorrespondenceStoreError, StoreError,
};

pub(super) fn verify(
    store: &Store,
    input: &setup::Input,
    proposal: &ResultSchoolSourceProposalV1,
) -> TestResult {
    let before = setup::state(store)?;
    for (case, detail) in [
        (0, "original row/statement bytes"),
        (1, "capture root schema"),
        (2, "active person attachment"),
        (3, "school provider identity"),
    ] {
        let inspection =
            store.inspect_result_school_correspondence_v1(AttachmentReplayGrantV1::new())?;
        let mut source = proposal.claim().clone();
        match case {
            0 => {
                source.statement.statement_offset = source
                    .statement
                    .statement_offset
                    .checked_add(1)
                    .ok_or("span overflow")?
            }
            1 => source.statement.result_root = source.target.provider_school_root.clone(),
            2 => source.person.accepted_group_digest = source.statement.body.digest.clone(),
            3 => {
                source.target.school = CanonicalSchool::mint(
                    census_domain::UsJurisdiction::Michigan,
                    "Different school",
                    "differentschool",
                    Some("Novi"),
                )
            }
            _ => return Err("unknown refusal case".into()),
        }
        let command = review::command(
            &inspection,
            &source,
            &format!("school-correspondence-refusal-{case}"),
            ResultSchoolActionV1::Accept {
                prior: prior(store)?,
            },
        )?;
        let context = proposal.borrowed_context(&mut AttachmentReplayGrantV1::new())?;
        let result = store.prepare_result_school_correspondence_v1(
            command,
            ResultSchoolCorrespondencePreparationV1::New {
                inspection,
                context: &context,
            },
        );
        match result {
            Err(StoreError::ResultSchoolCorrespondence(
                ResultSchoolCorrespondenceStoreError::SourceMismatch(found),
            )) => check!(eq; found, detail),
            Err(error) => return Err(format!("wrong source refusal case {case}: {error}").into()),
            Ok(_) => return Err(format!("invalid source case {case} prepared").into()),
        }
        check!(eq; setup::state(store)?, before);
    }
    source_grant_refusal(store, input)?;
    foreign(store, proposal)?;
    check!(eq; setup::state(store)?, before);
    Ok(())
}

fn prior(store: &Store) -> TestResult<ResultSchoolPredecessorV1> {
    let record = review::record(store)?;
    Ok(ResultSchoolPredecessorV1::After(
        ResultSchoolRecordReferenceV1 {
            operation: record.command.operation,
            digest: record.record_digest,
        },
    ))
}

fn source_grant_refusal(store: &Store, input: &setup::Input) -> TestResult {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let provider =
        read_recorded_provider_school(&snapshot, &input.school_packet.reference()?, &mut grant)?
            .ok_or("school source")?;
    let plan = discover_current_html_results(&current, &mut grant)?;
    let scope = scope()?;
    let result = plan.with_group(0, &mut grant, |linked, _| {
        Ok(build_claim(
            &current,
            &linked,
            0,
            &provider,
            scope,
            &mut AttachmentReplayGrantV1::new(),
        )
        .err())
    })?;
    match result {
        Some(crate::CrawlError::Store(StoreError::EvidenceSubjectAttachment(
            census_store::EvidenceSubjectAttachmentStoreError::Domain(
                EvidenceSubjectAttachmentError::Invalid(detail),
            ),
        ))) => {
            check!(eq; detail, "grant continuation usage regressed");
        }
        Some(error) => return Err(format!("wrong source grant refusal: {error}").into()),
        None => return Err("foreign grant built source proposal".into()),
    }
    Ok(())
}

pub(super) fn prepare_stale<'a>(
    store: &'a Store,
    proposal: &ResultSchoolSourceProposalV1,
) -> TestResult<PreparedResultSchoolCorrespondenceV1<'a>> {
    prepare(store, proposal, "school-correspondence-stale-prepared")
}

fn prepare<'a>(
    store: &'a Store,
    proposal: &ResultSchoolSourceProposalV1,
    operation: &str,
) -> TestResult<PreparedResultSchoolCorrespondenceV1<'a>> {
    let inspection =
        store.inspect_result_school_correspondence_v1(AttachmentReplayGrantV1::new())?;
    let command = review::command(
        &inspection,
        proposal.claim(),
        operation,
        ResultSchoolActionV1::Accept {
            prior: prior(store)?,
        },
    )?;
    let context = proposal.borrowed_context(&mut AttachmentReplayGrantV1::new())?;
    Ok(store.prepare_result_school_correspondence_v1(
        command,
        ResultSchoolCorrespondencePreparationV1::New {
            inspection,
            context: &context,
        },
    )?)
}

pub(super) fn apply_stale(
    store: &Store,
    prepared: PreparedResultSchoolCorrespondenceV1<'_>,
) -> TestResult {
    let before = setup::state(store)?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    let expected = (
        prepared.command().evidence_generation,
        prepared.command().derived_generation,
    );
    match store.apply_result_school_correspondence_v1(prepared, &approved, &digest) {
        Err(StoreError::RevisionMoved {
            operation,
            expected_evidence,
            actual_evidence,
            expected_derived,
            actual_derived,
        }) => {
            check!(eq; operation, "school-correspondence-stale-prepared");
            check!(eq; (expected_evidence, expected_derived), expected);
            check!(eq; (actual_evidence, actual_derived), (store.evidence_generation(), store.snapshot().derived_generation()));
        }
        Err(error) => return Err(format!("wrong stale preparation refusal: {error}").into()),
        Ok(_) => return Err("stale preparation applied after revoke".into()),
    }
    check!(eq; setup::state(store)?, before);
    Ok(())
}

fn foreign(store: &Store, proposal: &ResultSchoolSourceProposalV1) -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("foreign");
    Store::create_public_identity_qualification_v3(&path)?;
    let (foreign, _writer) = Store::open_public_identity_qualification_v3(&path)?.into_parts();
    let before = setup::state(&foreign)?;
    let prepared = prepare(store, proposal, "school-correspondence-foreign-origin")?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match foreign.apply_result_school_correspondence_v1(prepared, &approved, &digest) {
        Err(StoreError::RevisionOriginMismatch { operation }) => {
            check!(eq; operation, "school-correspondence-foreign-origin")
        }
        Err(error) => return Err(format!("wrong foreign origin refusal: {error}").into()),
        Ok(_) => return Err("foreign origin applied source review".into()),
    }
    check!(eq; setup::state(&foreign)?, before);
    Ok(())
}
