use super::fixture::*;
use crate::evidence_subject_attachment::{
    AttachmentInspectionRequestV1, EvidenceSubjectAttachmentPreparationV1 as Preparation,
    EvidenceSubjectAttachmentStoreError as AttachmentError,
};
use crate::public_identity_tests::support::{retained_state, TestResult};
use crate::{Store, StoreError};
use census_domain::evidence_subject_attachment::*;
use serde_json::Value;

const TOKEN_KEY: &str = "capability:public-identity-input-revisions-v2";
mod support;
use support::*;

#[test]
fn raw_full_token_baseline_corruption_refuses_apply_even_when_capture_counters_and_generations_match(
) -> TestResult {
    let fixture = Fixture::new()?;
    let context = Context::synthetic("raw-token-baseline")?;
    let inspection = fixture.inspect()?;
    let command = context.command("raw-token-baseline", &inspection)?;
    let prepared = prepare(&fixture.store, inspection, command, &context)?;
    let approval = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    let evidence = fixture.store.evidence_generation();
    let derived = fixture.store.derived_generation();
    let raw = fixture
        .store
        .meta
        .get(TOKEN_KEY)?
        .ok_or("enabled input token absent")?;
    let mut token: Value = serde_json::from_slice(&raw)?;
    let counters = token
        .get("counters")
        .ok_or("input counters absent")?
        .clone();
    let baseline = token
        .get("baseline_evidence_generation")
        .and_then(Value::as_u64)
        .ok_or("baseline absent")?;
    let changed_baseline = baseline.checked_add(1).ok_or("baseline overflow")?;
    check!(changed_baseline <= evidence);
    *token
        .get_mut("baseline_evidence_generation")
        .ok_or("baseline absent")? = Value::from(changed_baseline);
    fixture
        .store
        .meta
        .insert(TOKEN_KEY, serde_json::to_vec(&token)?)?;
    check!(eq; token.get("counters"), Some(&counters));
    check!(eq; fixture.store.evidence_generation(), evidence);
    check!(eq; fixture.store.derived_generation(), derived);
    let before = retained_state(&fixture.store)?;
    check!(matches!(
        fixture
            .store
            .apply_evidence_subject_attachment_v1(prepared, &approval, &digest),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::InputMoved
        ))
    ));
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn raw_stored_input_role_swap_refuses_without_rewriting_the_historical_record_or_witness(
) -> TestResult {
    let (fixture, _context, original) = committed("raw-role-swap")?;
    let mut altered = entry(&fixture.store)?;
    let capture = altered
        .pointer("/payload/inputs/capture")
        .ok_or("capture role absent")?
        .clone();
    let token = altered
        .pointer("/payload/inputs/token")
        .ok_or("token role absent")?
        .clone();
    *altered
        .pointer_mut("/payload/inputs/capture")
        .ok_or("capture role absent")? = token;
    *altered
        .pointer_mut("/payload/inputs/token")
        .ok_or("token role absent")? = capture;
    replace_entry_preserving_record(&fixture.store, &altered, &original)?;
    let before = retained_state(&fixture.store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(
        matches!(fixture.store.snapshot().read_evidence_subject_attachment_references_v1(&mut grant),
            Err(StoreError::Decode { key, .. }) if key == "attachment capture binding"
        )
    );
    poisoned_without_further_reads(&fixture.store, &mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn raw_stored_input_reference_outside_complete_context_refuses_before_object_resolution(
) -> TestResult {
    let (fixture, _context, original) = committed("raw-undeclared-input")?;
    let mut altered = entry(&fixture.store)?;
    let absent = reference(b"This input was never introduced or reviewed")?;
    check!(!original
        .command
        .reviewed_context
        .objects
        .as_slice()
        .contains(&absent));
    *altered
        .pointer_mut("/payload/inputs/token")
        .ok_or("token role absent")? = serde_json::to_value(absent)?;
    replace_entry_preserving_record(&fixture.store, &altered, &original)?;
    let before = retained_state(&fixture.store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        fixture
            .store
            .snapshot()
            .read_evidence_subject_attachment_references_v1(&mut grant),
        Err(StoreError::EvidenceSubjectAttachment(
            AttachmentError::Corrupt("entry input role absent from complete context")
        ))
    ));
    poisoned_without_further_reads(&fixture.store, &mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn raw_omitted_stored_input_token_is_not_recovered_from_the_unchanged_historical_witness(
) -> TestResult {
    let (fixture, _context, original) = committed("raw-omitted-input")?;
    let mut altered = entry(&fixture.store)?;
    altered
        .pointer_mut("/payload/inputs")
        .and_then(Value::as_object_mut)
        .ok_or("input role map absent")?
        .remove("token")
        .ok_or("token role absent")?;
    replace_entry_preserving_record(&fixture.store, &altered, &original)?;
    let before = retained_state(&fixture.store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    match fixture
        .store
        .snapshot()
        .read_evidence_subject_attachment_references_v1(&mut grant)
    {
        Err(StoreError::EvidenceSubjectAttachment(AttachmentError::Domain(
            EvidenceSubjectAttachmentError::Decoding(source),
        ))) => {
            check!(eq; source.classify(), serde_json::error::Category::Data);
            check!(source.to_string().starts_with("missing field `token`"));
        }
        Err(other) => {
            return Err(format!("expected missing-token schema refusal, got {other:?}").into())
        }
        Ok(_) => return Err("missing input token unexpectedly read".into()),
    }
    poisoned_without_further_reads(&fixture.store, &mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn raw_stored_token_role_cannot_point_at_validly_hashed_synthetic_document_bytes() -> TestResult {
    let (fixture, context, original) = committed("raw-malformed-token")?;
    let mut altered = entry(&fixture.store)?;
    let synthetic = &context
        .objects
        .first()
        .ok_or("synthetic document absent")?
        .reference;
    *altered
        .pointer_mut("/payload/inputs/token")
        .ok_or("token role absent")? = serde_json::to_value(synthetic)?;
    replace_entry_preserving_record(&fixture.store, &altered, &original)?;
    let before = retained_state(&fixture.store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(
        matches!(fixture.store.snapshot().read_evidence_subject_attachment_references_v1(&mut grant),
            Err(StoreError::Json { detail, source })
                if detail == "decoding capability:public-identity-input-revisions-v2"
                    && source.classify() == serde_json::error::Category::Data
        )
    );
    poisoned_without_further_reads(&fixture.store, &mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn raw_stored_capture_role_cannot_point_at_validly_hashed_synthetic_document_bytes() -> TestResult {
    let (fixture, context, original) = committed("raw-malformed-capture")?;
    let mut altered = entry(&fixture.store)?;
    let synthetic = &context
        .objects
        .first()
        .ok_or("synthetic document absent")?
        .reference;
    *altered
        .pointer_mut("/payload/inputs/capture")
        .ok_or("capture role absent")? = serde_json::to_value(synthetic)?;
    replace_entry_preserving_record(&fixture.store, &altered, &original)?;
    let before = retained_state(&fixture.store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(
        matches!(fixture.store.snapshot().read_evidence_subject_attachment_references_v1(&mut grant),
            Err(StoreError::Decode { key, source })
                if key == "attachment capture binding" && source.classify() == serde_json::error::Category::Data
        )
    );
    poisoned_without_further_reads(&fixture.store, &mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn raw_out_of_order_introductions_refuse_before_sorted_lookup_can_hide_them() -> TestResult {
    order_cases::raw_out_of_order_introductions_refuse_before_sorted_lookup_can_hide_them()
}

#[test]
fn raw_duplicate_across_prior_introduction_sets_refuses_with_both_historical_records_unchanged(
) -> TestResult {
    order_cases::raw_duplicate_across_prior_introduction_sets_refuses_with_both_historical_records_unchanged()
}

mod order_cases;
